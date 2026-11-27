//! Sharing objects between machines through a remote store.
//!
//! Commit metadata already lives in the shared PostgreSQL database, so only
//! file contents need to travel. The remote is a directory (typically a NAS
//! mount such as `/mnt/nas/buckets-objects`) laid out exactly like the local
//! `.buckets/objects` store. `push` copies every committed object the remote
//! lacks; `pull` brings the working files of a bucket up to its latest commit,
//! fetching only the objects that commit needs.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::args::{PullCommand, PushCommand, RemoteAction, RemoteCommand};
use crate::commands::BucketCommand;
use crate::data::bucket::{read_bucket_info, Bucket, BucketTrait};
use crate::data::manifest::{known_hashes_by_path, load_commit_with_store, referenced_hashes};
use crate::errors::BucketError;
use crate::postgres_db::get_database;
use crate::utils::object_store::{write_atomically, ObjectStore};
use crate::utils::runtime::RuntimeManager;
use crate::utils::scan::scan_bucket;
use crate::utils::utils::{find_bucket_path, find_bucket_repo};
use crate::CURRENT_DIR;

const REMOTE_FILE: &str = "remote.toml";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RemoteConfig {
    pub path: PathBuf,
}

fn repo_root() -> Result<PathBuf, BucketError> {
    let current_dir = CURRENT_DIR.with(|dir| dir.clone());
    let buckets_dir = find_bucket_repo(&current_dir).ok_or(BucketError::NotInRepo)?;
    buckets_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or(BucketError::NotInRepo)
}

fn remote_file(repo_root: &Path) -> PathBuf {
    repo_root.join(".buckets").join(REMOTE_FILE)
}

/// The configured remote: `BUCKETS_REMOTE` wins over `.buckets/remote.toml`.
pub fn load_remote(repo_root: &Path) -> Result<Option<RemoteConfig>, BucketError> {
    if let Ok(path) = std::env::var("BUCKETS_REMOTE") {
        if !path.trim().is_empty() {
            return Ok(Some(RemoteConfig {
                path: PathBuf::from(path),
            }));
        }
    }
    let file = remote_file(repo_root);
    if !file.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&file)?;
    let config: RemoteConfig = toml::from_str(&text)
        .map_err(|e| BucketError::from(format!("Invalid {}: {}", file.display(), e).as_str()))?;
    Ok(Some(config))
}

fn require_remote(repo_root: &Path) -> Result<ObjectStore, BucketError> {
    let config = load_remote(repo_root)?.ok_or_else(|| {
        BucketError::from("No remote store configured. Run `buckets remote set <path>` first.")
    })?;
    if !config.path.is_dir() {
        return Err(BucketError::from(
            format!(
                "Remote store {} is not reachable (is the NAS mounted?)",
                config.path.display()
            )
            .as_str(),
        ));
    }
    Ok(ObjectStore::at(config.path))
}

/// Every bucket directory in the repository with its metadata.
fn discover_buckets(repo_root: &Path) -> Vec<(Bucket, PathBuf)> {
    let mut found = Vec::new();
    for entry in WalkDir::new(repo_root)
        .max_depth(6)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            e.depth() == 0 || (name != ".buckets" && name != ".b" && name != ".git")
        })
        .filter_map(Result::ok)
    {
        if entry.file_type().is_dir() && entry.path().join(".b").join("info").is_file() {
            if let Ok(bucket) = read_bucket_info(entry.path()) {
                found.push((bucket, entry.path().to_path_buf()));
            }
        }
    }
    found
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{}", text),
        Err(e) => eprintln!("Error serializing to JSON: {}", e),
    }
}

pub struct Remote {
    args: RemoteCommand,
}

impl BucketCommand for Remote {
    type Args = RemoteCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let root = repo_root()?;
        match &self.args.action {
            RemoteAction::Set { path } => {
                let path = if path.is_absolute() {
                    path.clone()
                } else {
                    CURRENT_DIR.with(|dir| dir.join(path))
                };
                fs::create_dir_all(&path)?;
                let text = toml::to_string(&RemoteConfig { path: path.clone() })
                    .map_err(|e| BucketError::from(e.to_string().as_str()))?;
                write_atomically(&remote_file(&root), |w| {
                    std::io::Write::write_all(w, text.as_bytes())
                })?;
                println!("Remote store set to {}", path.display());
            }
            RemoteAction::Show => match load_remote(&root)? {
                Some(config) if self.args.shared.json => print_json(&config),
                Some(config) => println!("{}", config.path.display()),
                None if self.args.shared.json => println!("null"),
                None => println!("No remote store configured"),
            },
            RemoteAction::Unset => {
                let file = remote_file(&root);
                if file.exists() {
                    fs::remove_file(file)?;
                }
                println!("Remote store removed");
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Default)]
struct PushSummary {
    remote: String,
    objects_copied: usize,
    objects_already_there: usize,
    missing_locally: Vec<String>,
}

pub struct Push {
    args: PushCommand,
}

impl BucketCommand for Push {
    type Args = PushCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let root = repo_root()?;
        let remote = require_remote(&root)?;
        let local = ObjectStore::for_repo(&root);
        let buckets = discover_buckets(&root);
        let ids: Vec<Uuid> = buckets.iter().map(|(b, _)| b.id).collect();

        let hashes = RuntimeManager::block_on(async {
            let db = get_database().await?;
            referenced_hashes(&db, &ids).await
        })?;

        let mut summary = PushSummary {
            remote: remote.root().display().to_string(),
            ..Default::default()
        };
        let legacy_dirs: Vec<&PathBuf> = buckets.iter().map(|(_, p)| p).collect();
        for hash in hashes {
            if remote.contains(&hash) {
                summary.objects_already_there += 1;
                continue;
            }
            let source = local.locate(&hash, None).or_else(|| {
                legacy_dirs
                    .iter()
                    .find_map(|dir| local.locate(&hash, Some(dir)))
            });
            match source {
                Some(path) => {
                    remote.put_raw(&path, &hash)?;
                    summary.objects_copied += 1;
                }
                None => summary.missing_locally.push(hash),
            }
        }

        if self.args.shared.json {
            print_json(&summary);
        } else {
            println!(
                "Pushed {} object(s) to {} ({} already there)",
                summary.objects_copied, summary.remote, summary.objects_already_there
            );
            if !summary.missing_locally.is_empty() {
                println!(
                    "{} object(s) are not on this machine; push them from the machine that committed them.",
                    summary.missing_locally.len()
                );
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Default, Debug)]
pub struct PullSummary {
    pub bucket: String,
    pub commit_id: Option<String>,
    pub updated: Vec<String>,
    pub removed: Vec<String>,
    pub conflicts: Vec<String>,
    pub objects_fetched: usize,
}

pub struct Pull {
    args: PullCommand,
}

impl BucketCommand for Pull {
    type Args = PullCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let root = repo_root()?;
        let remote = load_remote(&root)?
            .filter(|c| c.path.is_dir())
            .map(|c| ObjectStore::at(c.path));
        let current_dir = CURRENT_DIR.with(|dir| dir.clone());

        // Inside a bucket: pull that bucket. At the repository root: pull all of them.
        let targets: Vec<(Bucket, PathBuf)> = match find_bucket_path(&current_dir) {
            Some(path) => vec![(Bucket::from_meta_data(&path)?, path)],
            None => discover_buckets(&root),
        };

        let mut summaries = Vec::new();
        for (bucket, path) in targets {
            summaries.push(pull_bucket(
                &root,
                remote.as_ref(),
                &bucket,
                &path,
                self.args.force,
            )?);
        }

        if self.args.shared.json {
            print_json(&summaries);
        } else {
            for s in &summaries {
                match &s.commit_id {
                    None => println!("{}: no commits yet", s.bucket),
                    Some(id) => println!(
                        "{}: at {} ({} updated, {} removed, {} object(s) fetched)",
                        s.bucket,
                        id,
                        s.updated.len(),
                        s.removed.len(),
                        s.objects_fetched
                    ),
                }
                for c in &s.conflicts {
                    println!("  kept local edit: {} (use --force to overwrite)", c);
                }
            }
        }
        if summaries.iter().any(|s| !s.conflicts.is_empty()) {
            return Err(BucketError::from(
                "Some files have local edits and were not updated",
            ));
        }
        Ok(())
    }
}

/// Remove directories left empty by deleting `file`, up to (not including) `root`.
fn prune_empty_parents(file: &Path, root: &Path) {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d == root || !d.starts_with(root) || fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

/// Bring one bucket's working files up to its latest commit.
pub fn pull_bucket(
    repo_root: &Path,
    remote: Option<&ObjectStore>,
    bucket: &Bucket,
    bucket_path: &Path,
    force: bool,
) -> Result<PullSummary, BucketError> {
    let local = ObjectStore::for_repo(repo_root);
    let mut summary = PullSummary {
        bucket: bucket.name.clone(),
        ..Default::default()
    };

    let (resolved, known) = RuntimeManager::block_on(async {
        let db = get_database().await?;
        let resolved = load_commit_with_store(&*db, bucket.id, None).await?;
        let known = known_hashes_by_path(&db, bucket.id).await?;
        Ok::<_, BucketError>((resolved, known))
    })?;
    let Some(resolved) = resolved else {
        return Ok(summary);
    };
    summary.commit_id = Some(resolved.id.to_string());

    let target: BTreeMap<String, String> = resolved
        .data
        .files
        .iter()
        .map(|f| (f.name.clone(), f.hash.to_hex().to_string()))
        .collect();
    let local_files: BTreeMap<String, String> = scan_bucket(bucket_path)?
        .into_iter()
        .map(|f| (f.path, f.hash.to_hex().to_string()))
        .collect();
    let is_known = |path: &str, hash: &str| {
        known
            .get(path)
            .map(|hashes| hashes.iter().any(|h| h == hash))
            .unwrap_or(false)
    };

    // Fetch the objects we will need before touching any working file.
    let mut needed: BTreeSet<String> = BTreeSet::new();
    for (path, hash) in &target {
        match local_files.get(path) {
            Some(current) if current == hash => {}
            Some(current) if !force && !is_known(path, current) => {
                summary.conflicts.push(path.clone())
            }
            _ => {
                needed.insert(hash.clone());
            }
        }
    }
    for hash in &needed {
        if local.locate(hash, Some(bucket_path)).is_some() {
            continue;
        }
        let source = remote.and_then(|r| r.locate(hash, None)).ok_or_else(|| {
            BucketError::from(
                format!(
                    "Object {} is not available locally{}",
                    hash,
                    if remote.is_some() {
                        " or in the remote store (was it pushed?)"
                    } else {
                        " and no remote store is configured"
                    }
                )
                .as_str(),
            )
        })?;
        local.put_raw(&source, hash)?;
        summary.objects_fetched += 1;
    }

    // Remove files deleted in the latest commit first, so a path that turns
    // from a file into a directory (or back) can be restored afterwards.
    // Files this bucket never committed are untracked and always left alone.
    for (path, hash) in &local_files {
        if target.contains_key(path) || !known.contains_key(path) {
            continue;
        }
        if force || is_known(path, hash) {
            let full = bucket_path.join(path);
            fs::remove_file(&full)?;
            prune_empty_parents(&full, bucket_path);
            summary.removed.push(path.clone());
        } else {
            summary.conflicts.push(path.clone());
        }
    }

    for (path, hash) in &target {
        if summary.conflicts.contains(path) || local_files.get(path) == Some(hash) {
            continue;
        }
        let dest = bucket_path.join(path);
        // Something still in the way (a kept local file where a directory must
        // go, or a non-empty directory where a file must go) is a conflict.
        let blocked_by_file = Path::new(path)
            .ancestors()
            .skip(1)
            .filter(|a| !a.as_os_str().is_empty())
            .any(|a| bucket_path.join(a).is_file());
        if blocked_by_file || dest.is_dir() {
            summary.conflicts.push(path.clone());
            continue;
        }
        local.restore(hash, &dest, Some(bucket_path))?;
        summary.updated.push(path.clone());
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn remote_env_overrides_file() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".buckets")).unwrap();
        fs::write(remote_file(dir.path()), "path = \"/srv/nas/objects\"\n").unwrap();
        std::env::remove_var("BUCKETS_REMOTE");
        assert_eq!(
            load_remote(dir.path()).unwrap().unwrap().path,
            PathBuf::from("/srv/nas/objects")
        );
        std::env::set_var("BUCKETS_REMOTE", "/other");
        assert_eq!(
            load_remote(dir.path()).unwrap().unwrap().path,
            PathBuf::from("/other")
        );
        std::env::remove_var("BUCKETS_REMOTE");
    }

    #[test]
    fn discovers_nested_buckets() {
        let dir = tempdir().unwrap();
        for (name, rel) in [("art", "art"), ("tex", "stage/tex")] {
            let b = dir.path().join(rel).join(".b");
            fs::create_dir_all(&b).unwrap();
            fs::write(
                b.join("info"),
                format!(
                    "id = \"{}\"\nname = \"{}\"\nrelative_bucket_path = \"{}\"\n",
                    Uuid::new_v4(),
                    name,
                    rel
                ),
            )
            .unwrap();
        }
        let mut names: Vec<_> = discover_buckets(dir.path())
            .into_iter()
            .map(|(b, _)| b.name)
            .collect();
        names.sort();
        assert_eq!(names, vec!["art", "tex"]);
    }
}
