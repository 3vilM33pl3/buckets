//! `buckets stash`: set uncommitted changes aside and restore the last commit.
//!
//! Stashed file contents go into the shared object store; the list of stashes
//! is kept per bucket in `.b/stash.json` (newest last). Nothing is written to
//! the database, so stashes stay private to the machine that made them.

use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::args::{StashAction, StashCommand};
use crate::commands::commit::{list_bucket_files, Commit};
use crate::commands::BucketCommand;
use crate::data::bucket::{Bucket, BucketTrait};
use crate::data::commit::CommitStatus;
use crate::errors::BucketError;
use crate::utils::object_store::{remove_file_and_prune, write_atomically, ObjectStore};
use crate::utils::runtime::RuntimeManager;
use crate::utils::utils::find_bucket_path;
use crate::CURRENT_DIR;

const STASH_FILE: &str = "stash.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StashEntry {
    pub id: String,
    pub message: String,
    pub created_at: String,
    pub files: Vec<StashedFile>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StashedFile {
    pub path: String,
    /// Content hash, or `None` when the stash records a deletion.
    pub hash: Option<String>,
}

fn stash_path(bucket_path: &Path) -> std::path::PathBuf {
    bucket_path.join(".b").join(STASH_FILE)
}

fn load_stashes(bucket_path: &Path) -> Result<Vec<StashEntry>, BucketError> {
    let path = stash_path(bucket_path);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(&path)?;
    serde_json::from_slice(&bytes)
        .map_err(|e| BucketError::from(format!("Corrupt {}: {}", path.display(), e).as_str()))
}

fn save_stashes(bucket_path: &Path, stashes: &[StashEntry]) -> Result<(), BucketError> {
    let bytes = serde_json::to_vec_pretty(stashes)
        .map_err(|e| BucketError::from(e.to_string().as_str()))?;
    write_atomically(&stash_path(bucket_path), |w| w.write_all(&bytes))?;
    Ok(())
}

pub struct Stash {
    args: StashCommand,
}

impl BucketCommand for Stash {
    type Args = StashCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let current_dir = CURRENT_DIR.with(|dir| dir.clone());
        let bucket_path = find_bucket_path(&current_dir).ok_or(BucketError::NotInBucket)?;
        let bucket = Bucket::from_meta_data(&bucket_path)?;
        let json = self.args.shared.json;

        match self
            .args
            .action
            .clone()
            .unwrap_or(StashAction::Save { message: None })
        {
            StashAction::Save { message } => save(&bucket, &bucket_path, message, json),
            StashAction::Pop { force } => pop(&bucket, &bucket_path, force, json),
            StashAction::List => {
                let stashes = load_stashes(&bucket_path)?;
                if json {
                    print_json(&stashes.iter().rev().collect::<Vec<_>>());
                } else if stashes.is_empty() {
                    println!("No stashes");
                } else {
                    for (i, s) in stashes.iter().rev().enumerate() {
                        println!(
                            "stash@{{{}}}  {}  {} ({} files)",
                            i,
                            s.created_at,
                            s.message,
                            s.files.len()
                        );
                    }
                }
                Ok(())
            }
            StashAction::Drop => {
                let mut stashes = load_stashes(&bucket_path)?;
                let dropped = stashes
                    .pop()
                    .ok_or_else(|| BucketError::from("No stash to drop"))?;
                save_stashes(&bucket_path, &stashes)?;
                println!("Dropped stash: {}", dropped.message);
                Ok(())
            }
        }
    }
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(text) => println!("{}", text),
        Err(e) => eprintln!("Error serializing to JSON: {}", e),
    }
}

fn save(
    bucket: &Bucket,
    bucket_path: &Path,
    message: Option<String>,
    json: bool,
) -> Result<(), BucketError> {
    let store = ObjectStore::for_path(bucket_path).ok_or(BucketError::NotInRepo)?;
    let current = list_bucket_files(bucket_path)?;
    let previous = RuntimeManager::block_on(Commit::load_last_commit_async(bucket.id))?;

    let changes = match &previous {
        Some(prev) => current.compare(prev).unwrap_or_default(),
        None => current.files,
    };
    if changes.is_empty() {
        println!("No local changes to stash");
        return Ok(());
    }

    // 1. Save the changed contents.
    let mut files = Vec::new();
    for change in &changes {
        match change.status {
            CommitStatus::Deleted => files.push(StashedFile {
                path: change.name.clone(),
                hash: None,
            }),
            _ => {
                let hex = change.hash.to_hex().to_string();
                store.put_file_verified(&bucket_path.join(&change.name), &hex)?;
                files.push(StashedFile {
                    path: change.name.clone(),
                    hash: Some(hex),
                });
            }
        }
    }
    let entry = StashEntry {
        id: Uuid::new_v4().to_string(),
        message: message.unwrap_or_else(|| format!("{} change(s)", files.len())),
        created_at: chrono::Utc::now().to_rfc3339(),
        files,
    };
    let mut stashes = load_stashes(bucket_path)?;
    stashes.push(entry.clone());
    save_stashes(bucket_path, &stashes)?;

    // 2. Put the working tree back to the last commit. Remove added files
    // first, so a path that changed between file and directory can be restored.
    for change in &changes {
        if !matches!(
            change.status,
            CommitStatus::Modified | CommitStatus::Deleted
        ) {
            remove_file_and_prune(&bucket_path.join(&change.name), bucket_path)?;
        }
    }
    for change in &changes {
        match change.status {
            CommitStatus::Modified => change.restore_version(bucket_path, &change.previous_hash)?,
            CommitStatus::Deleted => change.restore_version(bucket_path, &change.hash)?,
            _ => {}
        }
    }

    if json {
        print_json(&entry);
    } else {
        println!("Stashed {} file(s): {}", entry.files.len(), entry.message);
    }
    Ok(())
}

fn pop(bucket: &Bucket, bucket_path: &Path, force: bool, json: bool) -> Result<(), BucketError> {
    let mut stashes = load_stashes(bucket_path)?;
    let entry = stashes
        .last()
        .cloned()
        .ok_or_else(|| BucketError::from("No stash to pop"))?;

    if !force {
        let current = list_bucket_files(bucket_path)?;
        let previous = RuntimeManager::block_on(Commit::load_last_commit_async(bucket.id))?;
        let dirty = match &previous {
            Some(prev) => current.compare(prev).is_some(),
            None => !current.files.is_empty(),
        };
        if dirty {
            return Err(BucketError::from(
                "The bucket has uncommitted changes. Commit or stash them first, or use --force.",
            ));
        }
    }

    let store = ObjectStore::for_path(bucket_path).ok_or(BucketError::NotInRepo)?;
    // Removals first, then restores, for the same reason as in `save`.
    for file in entry.files.iter().filter(|f| f.hash.is_none()) {
        remove_file_and_prune(&bucket_path.join(&file.path), bucket_path)?;
    }
    for file in &entry.files {
        if let Some(hash) = &file.hash {
            store.restore(hash, &bucket_path.join(&file.path), Some(bucket_path))?;
        }
    }
    stashes.pop();
    save_stashes(bucket_path, &stashes)?;

    if json {
        print_json(&entry);
    } else {
        println!(
            "Applied stash: {} ({} file(s))",
            entry.message,
            entry.files.len()
        );
    }
    Ok(())
}
