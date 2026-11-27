use crate::args::{LockCommand, LocksCommand, UnlockCommand};
use crate::commands::BucketCommand;
use crate::data::bucket::{Bucket, BucketTrait};
use crate::data::lock::{current_owner, normalize_path, FileLock, LockOutcome};
use crate::errors::BucketError;
use crate::postgres_db::get_database;
use crate::utils::runtime::RuntimeManager;
use crate::utils::utils::find_bucket_path;
use crate::CURRENT_DIR;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Current bucket and its root directory.
fn current_bucket() -> Result<(Bucket, PathBuf), BucketError> {
    let current_dir = CURRENT_DIR.with(|dir| dir.clone());
    let bucket_path = find_bucket_path(&current_dir).ok_or(BucketError::NotInBucket)?;
    let bucket = Bucket::from_meta_data(&bucket_path)?;
    Ok((bucket, bucket_path))
}

/// Turn a user-supplied path (relative to the current directory, or already
/// relative to the bucket) into the bucket-relative form stored in locks.
pub(crate) fn bucket_relative(bucket_path: &Path, file: &str) -> String {
    let current_dir = CURRENT_DIR.with(|dir| dir.clone());
    let candidate = PathBuf::from(file);
    let absolute = if candidate.is_absolute() {
        candidate.clone()
    } else {
        current_dir.join(&candidate)
    };
    let relative = absolute
        .strip_prefix(bucket_path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| file.to_string());
    normalize_path(&relative)
}

#[derive(Serialize)]
struct LockResult {
    file: String,
    status: String,
    owner: String,
}

fn print_results(json: bool, results: &[LockResult]) {
    if json {
        match serde_json::to_string_pretty(results) {
            Ok(text) => println!("{}", text),
            Err(e) => eprintln!("Error serializing to JSON: {}", e),
        }
    } else {
        for r in results {
            println!("{}: {} ({})", r.status, r.file, r.owner);
        }
    }
}

pub struct Lock {
    args: LockCommand,
}

impl BucketCommand for Lock {
    type Args = LockCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let (bucket, bucket_path) = current_bucket()?;
        let owner = current_owner();
        let files: Vec<String> = self
            .args
            .files
            .iter()
            .map(|f| bucket_relative(&bucket_path, f))
            .collect();

        let results = RuntimeManager::block_on(async {
            let db = get_database().await?;
            let mut results = Vec::new();
            for file in &files {
                let outcome = FileLock::acquire(&db, bucket.id, file, &owner).await?;
                let (status, holder) = match outcome {
                    LockOutcome::Acquired => ("locked", owner.clone()),
                    LockOutcome::AlreadyYours => ("already locked by you", owner.clone()),
                    LockOutcome::HeldBy(other) => ("held by someone else", other),
                };
                results.push(LockResult {
                    file: file.clone(),
                    status: status.to_string(),
                    owner: holder,
                });
            }
            Ok::<_, BucketError>(results)
        })?;

        print_results(self.args.shared.json, &results);
        let refused: Vec<&LockResult> = results
            .iter()
            .filter(|r| r.status == "held by someone else")
            .collect();
        if refused.is_empty() {
            Ok(())
        } else {
            Err(BucketError::from(
                format!(
                    "{} file(s) are locked by someone else: {}",
                    refused.len(),
                    refused
                        .iter()
                        .map(|r| format!("{} ({})", r.file, r.owner))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .as_str(),
            ))
        }
    }
}

pub struct Unlock {
    args: UnlockCommand,
}

impl BucketCommand for Unlock {
    type Args = UnlockCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let (bucket, bucket_path) = current_bucket()?;
        let owner = current_owner();
        let force = self.args.force;
        let files: Vec<String> = self
            .args
            .files
            .iter()
            .map(|f| bucket_relative(&bucket_path, f))
            .collect();

        let results = RuntimeManager::block_on(async {
            let db = get_database().await?;
            let mut results = Vec::new();
            for file in &files {
                let released = FileLock::release(&db, bucket.id, file, &owner, force).await?;
                results.push(match released {
                    Some(previous) => LockResult {
                        file: file.clone(),
                        status: "unlocked".to_string(),
                        owner: previous,
                    },
                    None => LockResult {
                        file: file.clone(),
                        status: "not locked".to_string(),
                        owner: "-".to_string(),
                    },
                });
            }
            Ok::<_, BucketError>(results)
        })?;
        print_results(self.args.shared.json, &results);
        Ok(())
    }
}

pub struct Locks {
    args: LocksCommand,
}

impl BucketCommand for Locks {
    type Args = LocksCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        let (bucket, _) = current_bucket()?;
        let locks = RuntimeManager::block_on(async {
            let db = get_database().await?;
            FileLock::list_for_bucket(&db, bucket.id).await
        })?;

        if self.args.shared.json {
            match serde_json::to_string_pretty(&locks) {
                Ok(text) => println!("{}", text),
                Err(e) => eprintln!("Error serializing to JSON: {}", e),
            }
        } else if locks.is_empty() {
            println!("No locked files in bucket {}", bucket.name);
        } else {
            for lock in &locks {
                println!(
                    "{}  {}  (since {})",
                    lock.file_path,
                    lock.owner,
                    lock.created_at.format("%Y-%m-%d %H:%M")
                );
            }
        }
        Ok(())
    }
}
