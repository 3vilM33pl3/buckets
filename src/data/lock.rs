//! Exclusive file locks for assets that cannot be merged.
//!
//! A lock is a row in the shared `locks` table keyed by bucket and file path.
//! While someone holds a lock, `buckets commit` refuses to record changes to
//! that file for anyone else. Owners are identified by `BUCKETS_USER` when
//! set (agents can use e.g. `agent-lakepalace`), otherwise `user@host`.

use crate::errors::BucketError;
use crate::postgres_db::DatabaseManager;
use chrono::NaiveDateTime;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileLock {
    pub bucket_id: Uuid,
    pub file_path: String,
    pub owner: String,
    pub created_at: NaiveDateTime,
}

/// Result of trying to take a lock.
#[derive(Debug, PartialEq, Eq)]
pub enum LockOutcome {
    Acquired,
    AlreadyYours,
    HeldBy(String),
}

/// Identity used for lock ownership.
pub fn current_owner() -> String {
    if let Ok(user) = std::env::var("BUCKETS_USER") {
        if !user.trim().is_empty() {
            return user.trim().to_string();
        }
    }
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let host = std::env::var("HOSTNAME")
        .ok()
        .filter(|h| !h.trim().is_empty())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|h| h.trim().to_string())
                .filter(|h| !h.is_empty())
        })
        .unwrap_or_else(|| "localhost".to_string());
    format!("{}@{}", user, host)
}

/// Normalise a path to the form stored in the database (forward slashes, no `./`).
pub fn normalize_path(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    normalized
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

impl FileLock {
    pub async fn acquire(
        db: &DatabaseManager,
        bucket_id: Uuid,
        file_path: &str,
        owner: &str,
    ) -> Result<LockOutcome, BucketError> {
        let path = normalize_path(file_path);
        let inserted = db
            .execute(
                "INSERT INTO locks (bucket_id, file_path, owner) VALUES ($1, $2, $3)
                 ON CONFLICT (bucket_id, file_path) DO NOTHING",
                &[&bucket_id, &path, &owner],
            )
            .await?;
        if inserted == 1 {
            return Ok(LockOutcome::Acquired);
        }
        match Self::get(db, bucket_id, &path).await? {
            Some(lock) if lock.owner == owner => Ok(LockOutcome::AlreadyYours),
            Some(lock) => Ok(LockOutcome::HeldBy(lock.owner)),
            // Released between the insert and the read: try once more.
            None => {
                let retried = db
                    .execute(
                        "INSERT INTO locks (bucket_id, file_path, owner) VALUES ($1, $2, $3)
                         ON CONFLICT (bucket_id, file_path) DO NOTHING",
                        &[&bucket_id, &path, &owner],
                    )
                    .await?;
                if retried == 1 {
                    Ok(LockOutcome::Acquired)
                } else {
                    Err(BucketError::from(
                        "Lock is being changed concurrently, try again",
                    ))
                }
            }
        }
    }

    /// Release a lock. Without `force` only the owner may release it.
    /// Returns the previous owner if a lock was removed.
    pub async fn release(
        db: &DatabaseManager,
        bucket_id: Uuid,
        file_path: &str,
        owner: &str,
        force: bool,
    ) -> Result<Option<String>, BucketError> {
        let path = normalize_path(file_path);
        let Some(lock) = Self::get(db, bucket_id, &path).await? else {
            return Ok(None);
        };
        if lock.owner != owner && !force {
            return Err(BucketError::from(
                format!(
                    "'{}' is locked by {}. Use --force to break the lock.",
                    path, lock.owner
                )
                .as_str(),
            ));
        }
        db.execute(
            "DELETE FROM locks WHERE bucket_id = $1 AND file_path = $2",
            &[&bucket_id, &path],
        )
        .await?;
        Ok(Some(lock.owner))
    }

    pub async fn get(
        db: &DatabaseManager,
        bucket_id: Uuid,
        file_path: &str,
    ) -> Result<Option<FileLock>, BucketError> {
        let path = normalize_path(file_path);
        let rows = db
            .query(
                "SELECT bucket_id, file_path, owner, created_at FROM locks
                 WHERE bucket_id = $1 AND file_path = $2",
                &[&bucket_id, &path],
            )
            .await?;
        Ok(rows.first().map(row_to_lock))
    }

    pub async fn list_for_bucket(
        db: &DatabaseManager,
        bucket_id: Uuid,
    ) -> Result<Vec<FileLock>, BucketError> {
        let rows = db
            .query(
                "SELECT bucket_id, file_path, owner, created_at FROM locks
                 WHERE bucket_id = $1 ORDER BY file_path",
                &[&bucket_id],
            )
            .await?;
        Ok(rows.iter().map(row_to_lock).collect())
    }
}

fn row_to_lock(row: &tokio_postgres::Row) -> FileLock {
    FileLock {
        bucket_id: row.get(0),
        file_path: row.get(1),
        owner: row.get(2),
        created_at: row.get(3),
    }
}

/// Files among `changed` that are locked by someone other than `owner`.
pub fn blocked_by_locks<'a>(
    locks: &'a [FileLock],
    changed: &[String],
    owner: &str,
) -> Vec<&'a FileLock> {
    locks
        .iter()
        .filter(|lock| lock.owner != owner)
        .filter(|lock| changed.iter().any(|c| normalize_path(c) == lock.file_path))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn lock(path: &str, owner: &str) -> FileLock {
        FileLock {
            bucket_id: Uuid::nil(),
            file_path: path.to_string(),
            owner: owner.to_string(),
            created_at: chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
        }
    }

    #[test]
    fn normalizes_paths() {
        assert_eq!(normalize_path("./maps/L_Island.umap"), "maps/L_Island.umap");
        assert_eq!(normalize_path("maps\\a.uasset"), "maps/a.uasset");
    }

    #[test]
    fn only_other_owners_block() {
        let locks = vec![lock("a.uasset", "me"), lock("b.uasset", "agent")];
        let changed = vec![
            "a.uasset".to_string(),
            "b.uasset".to_string(),
            "c.txt".to_string(),
        ];
        let blocked = blocked_by_locks(&locks, &changed, "me");
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].file_path, "b.uasset");
    }

    #[test]
    #[serial]
    fn owner_prefers_buckets_user() {
        std::env::set_var("BUCKETS_USER", "agent-7");
        assert_eq!(current_owner(), "agent-7");
        std::env::remove_var("BUCKETS_USER");
        assert!(current_owner().contains('@'));
    }
}
