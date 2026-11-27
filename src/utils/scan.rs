//! Fast scanning of a bucket's working files.
//!
//! Hashing hundreds of gigabytes on every `status` is too slow, so the result
//! of each scan is cached in `.b/index.json` keyed by path, size and
//! modification time. A file is only re-hashed when its size or mtime changed.
//! Files modified within [`RACY_WINDOW_NS`] of the previous scan are always
//! re-hashed, because a second write in the same timestamp tick would not be
//! visible in the mtime (the classic "racy git" problem).
//!
//! Files that do need hashing are hashed in parallel with a 1 MiB read buffer.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use blake3::Hash;
use serde::{Deserialize, Serialize};

use crate::utils::object_store::write_atomically;
use crate::utils::utils::find_files_excluding_top_level_b;

const INDEX_FILE: &str = "index.json";
const INDEX_VERSION: u32 = 2;
const READ_BUFFER: usize = 1 << 20;
/// Entries whose mtime is this close to the previous scan are re-hashed.
pub const RACY_WINDOW_NS: u128 = 2_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    /// Path relative to the bucket root, using the platform separator.
    pub path: String,
    pub hash: Hash,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct IndexFile {
    version: u32,
    written_at_ns: u128,
    entries: BTreeMap<String, IndexEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct IndexEntry {
    size: u64,
    mtime_ns: u128,
    /// Inode, device and change time (ctime) on Unix; creation time elsewhere.
    /// Catches a same-size replacement that kept the old modification time.
    identity: String,
    hash: String,
}

/// Statistics of the last scan, useful for tests and `--verbose` output.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScanStats {
    pub files: usize,
    pub hashed: usize,
}

fn index_path(bucket_path: &Path) -> PathBuf {
    bucket_path.join(".b").join(INDEX_FILE)
}

fn now_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn mtime_ns(meta: &fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(unix)]
fn identity(meta: &fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    format!(
        "{}:{}:{}.{}",
        meta.dev(),
        meta.ino(),
        meta.ctime(),
        meta.ctime_nsec()
    )
}

#[cfg(not(unix))]
fn identity(meta: &fs::Metadata) -> String {
    meta.created()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_default()
}

fn load_index(bucket_path: &Path) -> IndexFile {
    fs::read(index_path(bucket_path))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<IndexFile>(&bytes).ok())
        .filter(|index| index.version == INDEX_VERSION)
        .unwrap_or_default()
}

/// Hash a file with a large read buffer.
pub fn hash_file_fast(path: &Path) -> io::Result<Hash> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; READ_BUFFER];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize())
}

/// Bucket-relative path with `/` separators on every platform, so manifests
/// written on one OS compare equal to scans on another.
pub fn portable_path(relative: &Path) -> String {
    relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Scan every file in the bucket (excluding `.b`) and return path, hash and size,
/// sorted by path. Uses and refreshes the `.b/index.json` cache.
pub fn scan_bucket(bucket_path: &Path) -> io::Result<Vec<ScannedFile>> {
    scan_bucket_with_stats(bucket_path).map(|(files, _)| files)
}

pub fn scan_bucket_with_stats(bucket_path: &Path) -> io::Result<(Vec<ScannedFile>, ScanStats)> {
    if !bucket_path.is_dir() {
        // A missing directory simply has no files.
        return Ok((Vec::new(), ScanStats::default()));
    }
    let index = load_index(bucket_path);
    let scan_started = now_ns();

    let mut results: Vec<Option<ScannedFile>> = Vec::new();
    let mut new_entries: BTreeMap<String, IndexEntry> = BTreeMap::new();
    let mut to_hash: Vec<(usize, PathBuf, String, u64, (u128, String))> = Vec::new();

    let mut relative_paths = find_files_excluding_top_level_b(bucket_path);
    relative_paths.sort();

    for relative in relative_paths {
        let full = bucket_path.join(&relative);
        // Follow symlinks the same way hashing does; skip anything that is not a file.
        let meta = match fs::metadata(&full) {
            Ok(meta) if meta.is_file() => meta,
            _ => continue,
        };
        let path = portable_path(&relative);
        let size = meta.len();
        let mtime = mtime_ns(&meta);
        let ident = identity(&meta);

        let cached = index.entries.get(&path).and_then(|entry| {
            let fresh = entry.size == size
                && entry.mtime_ns == mtime
                && entry.identity == ident
                && mtime.saturating_add(RACY_WINDOW_NS) < index.written_at_ns;
            if fresh {
                Hash::from_hex(&entry.hash).ok()
            } else {
                None
            }
        });

        match cached {
            Some(hash) => {
                new_entries.insert(
                    path.clone(),
                    IndexEntry {
                        size,
                        mtime_ns: mtime,
                        identity: ident,
                        hash: hash.to_hex().to_string(),
                    },
                );
                results.push(Some(ScannedFile { path, hash, size }));
            }
            None => {
                to_hash.push((results.len(), full, path, size, (mtime, ident)));
                results.push(None);
            }
        }
    }

    let hashed_count = to_hash.len();
    let hashed = hash_in_parallel(&to_hash)?;
    for ((slot, _, path, size, (mtime, ident)), hash) in to_hash.into_iter().zip(hashed) {
        new_entries.insert(
            path.clone(),
            IndexEntry {
                size,
                mtime_ns: mtime,
                identity: ident,
                hash: hash.to_hex().to_string(),
            },
        );
        results[slot] = Some(ScannedFile { path, hash, size });
    }

    let files: Vec<ScannedFile> = results.into_iter().flatten().collect();

    // Best effort: a read-only bucket still scans, it just isn't cached.
    if bucket_path.join(".b").is_dir() {
        let index = IndexFile {
            version: INDEX_VERSION,
            written_at_ns: scan_started,
            entries: new_entries,
        };
        if let Ok(bytes) = serde_json::to_vec(&index) {
            let _ = write_atomically(&index_path(bucket_path), |w| w.write_all(&bytes));
        }
    }

    let stats = ScanStats {
        files: files.len(),
        hashed: hashed_count,
    };
    Ok((files, stats))
}

fn hash_in_parallel(
    jobs: &[(usize, PathBuf, String, u64, (u128, String))],
) -> io::Result<Vec<Hash>> {
    if jobs.is_empty() {
        return Ok(Vec::new());
    }
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(jobs.len())
        .max(1);
    let next = AtomicUsize::new(0);
    let output: Mutex<Vec<Option<io::Result<Hash>>>> =
        Mutex::new((0..jobs.len()).map(|_| None).collect());

    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= jobs.len() {
                    break;
                }
                let result = hash_file_fast(&jobs[i].1).map_err(|e| {
                    io::Error::new(
                        e.kind(),
                        format!("Failed to hash '{}': {}", jobs[i].1.display(), e),
                    )
                });
                if let Ok(mut out) = output.lock() {
                    out[i] = Some(result);
                }
            });
        }
    });

    let output = output
        .into_inner()
        .map_err(|_| io::Error::other("hashing thread panicked"))?;
    output
        .into_iter()
        .map(|r| r.unwrap_or_else(|| Err(io::Error::other("file was not hashed"))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_paths_use_forward_slashes() {
        let p: PathBuf = ["assets", "chars", "hero.blend"].iter().collect();
        assert_eq!(portable_path(&p), "assets/chars/hero.blend");
    }
    use tempfile::tempdir;

    fn make_bucket() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".b").join("storage")).unwrap();
        dir
    }

    #[test]
    fn scans_files_sorted_and_skips_dot_b() {
        let dir = make_bucket();
        fs::write(dir.path().join("b.txt"), "b").unwrap();
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub").join("a.txt"), "a").unwrap();
        fs::write(dir.path().join(".b").join("info"), "x").unwrap();

        let files = scan_bucket(dir.path()).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.path.clone()).collect();
        assert_eq!(
            names,
            vec![
                "b.txt".to_string(),
                format!("sub{}a.txt", std::path::MAIN_SEPARATOR)
            ]
        );
        assert_eq!(files[0].hash, blake3::hash(b"b"));
        assert_eq!(files[0].size, 1);
    }

    #[test]
    fn cache_skips_rehash_of_old_unchanged_files() {
        let dir = make_bucket();
        let file = dir.path().join("old.bin");
        fs::write(&file, "content").unwrap();
        // Backdate the file well outside the racy window.
        let old = SystemTime::now() - std::time::Duration::from_secs(60);
        File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(old)
            .unwrap();

        let (_, first) = scan_bucket_with_stats(dir.path()).unwrap();
        assert_eq!(first.hashed, 1);
        let (files, second) = scan_bucket_with_stats(dir.path()).unwrap();
        assert_eq!(second.hashed, 0, "unchanged file served from cache");
        assert_eq!(files[0].hash, blake3::hash(b"content"));

        // Changing the content (and size) forces a re-hash.
        fs::write(&file, "changed!").unwrap();
        let (files, third) = scan_bucket_with_stats(dir.path()).unwrap();
        assert_eq!(third.hashed, 1);
        assert_eq!(files[0].hash, blake3::hash(b"changed!"));
    }

    #[test]
    fn recently_modified_files_are_always_rehashed() {
        let dir = make_bucket();
        fs::write(dir.path().join("new.txt"), "aaaa").unwrap();
        scan_bucket(dir.path()).unwrap();
        // Same size, written immediately after the scan: must not be trusted.
        fs::write(dir.path().join("new.txt"), "bbbb").unwrap();
        let files = scan_bucket(dir.path()).unwrap();
        assert_eq!(files[0].hash, blake3::hash(b"bbbb"));
    }

    #[test]
    fn replaced_file_with_same_size_and_mtime_is_rehashed() {
        let dir = make_bucket();
        let file = dir.path().join("asset.bin");
        let old = SystemTime::now() - std::time::Duration::from_secs(60);
        let write_backdated = |content: &str| {
            fs::write(&file, content).unwrap();
            File::options()
                .write(true)
                .open(&file)
                .unwrap()
                .set_modified(old)
                .unwrap();
        };
        write_backdated("aaaa");
        scan_bucket(dir.path()).unwrap();
        // Same size, same mtime, different content: ctime has moved on.
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::remove_file(&file).unwrap();
        write_backdated("bbbb");
        let files = scan_bucket(dir.path()).unwrap();
        assert_eq!(files[0].hash, blake3::hash(b"bbbb"));
    }

    #[test]
    fn hash_file_fast_matches_blake3() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("big");
        let data: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        fs::write(&path, &data).unwrap();
        assert_eq!(hash_file_fast(&path).unwrap(), blake3::hash(&data));
    }
}
