//! Repository-level content-addressable object store.
//!
//! Objects are zstd-compressed file contents named by their BLAKE3 hash and
//! stored once per repository under `.buckets/objects/<first 2 hex>/<rest>`,
//! so identical files in different buckets (or different versions that happen
//! to match) are stored only once.
//!
//! Writes are crash safe: an object is compressed into a temporary file in
//! the same directory, flushed to disk with `fsync`, and then atomically
//! renamed into place. A reader therefore never sees a half-written object.
//!
//! Repositories created before the shared store kept objects in each bucket's
//! `.b/storage/<hash>`. Reads fall back to that location, so old history keeps
//! working without a migration step.

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::utils::compression::DEFAULT_COMPRESSION_LEVEL;

/// Directory name of the shared store inside `.buckets`.
pub const OBJECTS_DIR: &str = "objects";

#[derive(Debug, Clone)]
pub struct ObjectStore {
    root: PathBuf,
}

impl ObjectStore {
    /// Store rooted at an explicit directory (used for remotes such as a NAS mount).
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The shared store of the repository whose root directory is `repo_root`
    /// (the directory that contains `.buckets`).
    pub fn for_repo(repo_root: &Path) -> Self {
        Self::at(repo_root.join(".buckets").join(OBJECTS_DIR))
    }

    /// Find the store for the repository containing `path` (any directory inside it).
    pub fn for_path(path: &Path) -> Option<Self> {
        crate::utils::utils::find_bucket_repo(path)
            .and_then(|buckets_dir| buckets_dir.parent().map(Self::for_repo))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path of the object with the given hex hash.
    pub fn object_path(&self, hash: &str) -> PathBuf {
        let (prefix, rest) = if hash.len() > 2 {
            hash.split_at(2)
        } else {
            (hash, "")
        };
        self.root.join(prefix).join(rest)
    }

    pub fn contains(&self, hash: &str) -> bool {
        self.object_path(hash).is_file()
    }

    /// Compress `source` into the store under `hash`. Does nothing if the
    /// object is already present. Returns true when a new object was written.
    #[allow(dead_code)]
    pub fn put_file(&self, source: &Path, hash: &str) -> io::Result<bool> {
        let target = self.object_path(hash);
        if target.is_file() {
            return Ok(false);
        }
        let input = File::open(source).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Failed to open '{}': {}", source.display(), e),
            )
        })?;
        write_atomically(&target, |out| {
            zstd::stream::copy_encode(BufReader::new(input), out, DEFAULT_COMPRESSION_LEVEL)
        })?;
        Ok(true)
    }

    /// Like [`put_file`](Self::put_file), but hashes the bytes while compressing
    /// them and refuses to store the object if they do not match `hash`. This
    /// catches files that are being written to while a commit runs.
    pub fn put_file_verified(&self, source: &Path, hash: &str) -> io::Result<bool> {
        let target = self.object_path(hash);
        if target.is_file() {
            return Ok(false);
        }
        let input = File::open(source).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Failed to open '{}': {}", source.display(), e),
            )
        })?;
        write_atomically(&target, |out| {
            let mut reader = HashingReader {
                inner: BufReader::with_capacity(1 << 20, input),
                hasher: blake3::Hasher::new(),
            };
            zstd::stream::copy_encode(&mut reader, out, DEFAULT_COMPRESSION_LEVEL)?;
            let actual = reader.hasher.finalize().to_hex().to_string();
            if actual != hash {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "'{}' changed while it was being committed; commit again once it is saved",
                        source.display()
                    ),
                ));
            }
            Ok(())
        })?;
        Ok(true)
    }

    /// Copy an already-compressed object file into this store (used by push/pull).
    /// Returns true when a new object was written.
    pub fn put_raw(&self, compressed: &Path, hash: &str) -> io::Result<bool> {
        let target = self.object_path(hash);
        if target.is_file() {
            return Ok(false);
        }
        let mut input = File::open(compressed)?;
        write_atomically(&target, |out| io::copy(&mut input, out).map(|_| ()))?;
        Ok(true)
    }

    /// Locate an object, falling back to a bucket's legacy `.b/storage` directory.
    pub fn locate(&self, hash: &str, legacy_bucket: Option<&Path>) -> Option<PathBuf> {
        let path = self.object_path(hash);
        if path.is_file() {
            return Some(path);
        }
        let legacy = legacy_bucket?.join(".b").join("storage").join(hash);
        legacy.is_file().then_some(legacy)
    }

    /// Decompress object `hash` to `dest`, replacing it atomically.
    pub fn restore(&self, hash: &str, dest: &Path, legacy_bucket: Option<&Path>) -> io::Result<()> {
        let source = self.locate(hash, legacy_bucket).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Object {} for '{}' is missing from the store. Try `buckets pull`.",
                    hash,
                    dest.display()
                ),
            )
        })?;
        let input = File::open(&source)?;
        write_atomically(dest, |out| {
            zstd::stream::copy_decode(BufReader::new(input), out)
        })
    }

    /// All object hashes in this store (does not include legacy bucket storage).
    #[allow(dead_code)]
    pub fn list(&self) -> io::Result<Vec<String>> {
        let mut hashes = Vec::new();
        if !self.root.is_dir() {
            return Ok(hashes);
        }
        for prefix in fs::read_dir(&self.root)? {
            let prefix = prefix?;
            if !prefix.file_type()?.is_dir() {
                continue;
            }
            let prefix_name = prefix.file_name().to_string_lossy().to_string();
            if prefix_name.starts_with('.') {
                continue;
            }
            for entry in fs::read_dir(prefix.path())? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.file_type()?.is_file() && !name.starts_with('.') {
                    hashes.push(format!("{}{}", prefix_name, name));
                }
            }
        }
        Ok(hashes)
    }
}

struct HashingReader<R> {
    inner: R,
    hasher: blake3::Hasher,
}

impl<R: io::Read> io::Read for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }
}

/// A path recorded in a commit or stash must stay inside the bucket: not
/// absolute, no `.`/`..` or empty parts, whichever separator it uses. A file
/// literally named `..\\x` on Linux would otherwise become `../x` once
/// separators are normalised.
pub fn is_safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !Path::new(path).has_root()
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && path
            .split(['/', '\\'])
            .all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(':'))
}

/// Error for a stored path that would leave the bucket.
pub fn unsafe_path_error(path: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "Refusing to use '{}': the path would leave the bucket",
            path
        ),
    )
}

/// Refuse to write `dest` when a directory between `root` and it is a
/// symlink: writing would follow the link and could change a file outside
/// the bucket.
pub fn check_no_symlinked_parent(dest: &Path, root: &Path) -> io::Result<()> {
    let Ok(relative) = dest.strip_prefix(root) else {
        return Ok(());
    };
    let mut current = root.to_path_buf();
    let parents: Vec<_> = relative.components().collect();
    for component in parents.iter().take(parents.len().saturating_sub(1)) {
        current.push(component);
        if let Ok(meta) = fs::symlink_metadata(&current) {
            if meta.file_type().is_symlink() {
                return Err(io::Error::other(format!(
                    "{} is a symlink; refusing to write {} through it",
                    current.display(),
                    dest.display()
                )));
            }
        }
    }
    Ok(())
}

/// Delete a working file, then remove any parent directories (below `root`)
/// left empty, so a later restore can put a file where that directory was.
pub fn remove_file_and_prune(file: &Path, root: &Path) -> io::Result<()> {
    // Never delete through a symlinked folder: that would remove a file
    // outside the bucket.
    check_no_symlinked_parent(file, root)?;
    if file.is_file() || file.is_symlink() {
        fs::remove_file(file)?;
    }
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d == root || !d.starts_with(root) || fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
    Ok(())
}

/// Write a file via a temporary sibling, fsync it, then rename it over `dest`.
pub fn write_atomically<F>(dest: &Path, write: F) -> io::Result<()>
where
    F: FnOnce(&mut BufWriter<File>) -> io::Result<()>,
{
    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent)?;
    // A fixed-length name, so long asset names never push the temporary
    // file past the filesystem's 255-byte limit.
    let tmp = parent.join(format!(".buckets-{}.tmp", Uuid::new_v4().simple()));

    let result = (|| {
        let file = File::create(&tmp)?;
        let mut writer = BufWriter::with_capacity(1 << 20, file);
        write(&mut writer)?;
        writer.flush()?;
        let file = writer.into_inner().map_err(|e| e.into_error())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, dest)?;
        // Persist the rename itself. Not supported on every platform/filesystem,
        // so a failure here is not fatal.
        if let Ok(dir) = File::open(&parent) {
            let _ = dir.sync_all();
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_paths_must_stay_inside_the_bucket() {
        for ok in [
            "a.txt",
            "assets/model.blend",
            "deep/er/file.uasset",
            "dots.in.name",
        ] {
            assert!(is_safe_relative(ok), "{ok}");
        }
        for bad in [
            "",
            "/etc/passwd",
            "../x",
            "a/../../x",
            "..\\outside.txt",
            "a\\..\\..\\x",
            "./a",
            "a//b",
            "C:\\x",
            "\\\\server\\share",
        ] {
            assert!(!is_safe_relative(bad), "{bad}");
        }
    }
    use tempfile::tempdir;

    #[test]
    fn put_and_restore_round_trip() {
        let dir = tempdir().unwrap();
        let store = ObjectStore::at(dir.path().join("objects"));
        let src = dir.path().join("a.bin");
        fs::write(&src, b"hello asset").unwrap();
        let hash = blake3::hash(b"hello asset").to_hex().to_string();

        assert!(store.put_file(&src, &hash).unwrap());
        assert!(
            !store.put_file(&src, &hash).unwrap(),
            "second put is a no-op"
        );
        assert!(store.contains(&hash));
        assert!(store
            .object_path(&hash)
            .starts_with(dir.path().join("objects").join(&hash[..2])));

        let dest = dir.path().join("out").join("a.bin");
        store.restore(&hash, &dest, None).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"hello asset");
        assert_eq!(store.list().unwrap(), vec![hash]);
    }

    #[test]
    fn restore_falls_back_to_legacy_bucket_storage() {
        let dir = tempdir().unwrap();
        let bucket = dir.path().join("bucket");
        let legacy = bucket.join(".b").join("storage");
        fs::create_dir_all(&legacy).unwrap();
        let src = dir.path().join("src.txt");
        fs::write(&src, b"old").unwrap();
        crate::utils::compression::compress_file(&src, &legacy.join("abc123"), 3).unwrap();

        let store = ObjectStore::at(dir.path().join("objects"));
        let dest = bucket.join("src.txt");
        store.restore("abc123", &dest, Some(&bucket)).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"old");
        assert!(store.restore("missing", &dest, Some(&bucket)).is_err());
    }

    #[test]
    fn verified_put_rejects_mismatched_content() {
        let dir = tempdir().unwrap();
        let store = ObjectStore::at(dir.path().join("objects"));
        let src = dir.path().join("a.bin");
        fs::write(&src, b"now different").unwrap();
        let stale_hash = blake3::hash(b"what was hashed").to_hex().to_string();
        assert!(store.put_file_verified(&src, &stale_hash).is_err());
        assert!(!store.contains(&stale_hash));

        let good = blake3::hash(b"now different").to_hex().to_string();
        assert!(store.put_file_verified(&src, &good).unwrap());
    }

    #[test]
    fn atomic_write_leaves_no_temp_files() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("x");
        write_atomically(&dest, |w| w.write_all(b"1")).unwrap();
        let failed = write_atomically(&dest, |_| Err(io::Error::other("boom")));
        assert!(failed.is_err());
        assert_eq!(
            fs::read(&dest).unwrap(),
            b"1",
            "failed write keeps old file"
        );
        let names: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(names.len(), 1);
    }
}
