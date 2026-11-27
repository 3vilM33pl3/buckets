//! Reading the set of files recorded by a commit.
//!
//! From schema V4 on, every commit stores a full manifest: one `files` row for
//! every file in the bucket at that moment, with `commits.manifest = TRUE`.
//! Reading a commit is then a single query.
//!
//! Commits written by older versions only stored the files that changed and
//! have `manifest = FALSE`. To read one of those, the history is folded from
//! the most recent full manifest (or from the first commit) up to the target,
//! each commit's rows overriding earlier ones. Old commits never recorded
//! deletions, so a file deleted under an old version still shows up there;
//! the next commit made with this version records the real state.

use std::collections::BTreeMap;
use std::io::{Error, ErrorKind};

use async_trait::async_trait;
use blake3::Hash;
use uuid::Uuid;

use crate::data::commit::{Commit as CommitData, CommitStatus, CommittedFile};
use crate::errors::BucketError;
use crate::postgres_db::{get_database, DatabaseManager};

#[derive(Clone, Debug)]
pub struct CommitRow {
    pub id: Uuid,
    pub bucket: String,
    #[allow(dead_code)]
    pub message: String,
    pub timestamp: String,
    pub manifest: bool,
}

#[derive(Clone, Debug)]
pub struct CommitFileRow {
    pub id: Uuid,
    pub file_path: String,
    pub hash: String,
}

/// A resolved commit: its id plus every file path and hash it contains.
pub struct ResolvedCommit {
    pub id: Uuid,
    pub data: CommitData,
}

#[async_trait]
pub trait CommitStore: Send + Sync {
    /// All commits of a bucket, oldest first.
    async fn commits_for_bucket(&self, bucket_id: Uuid) -> Result<Vec<CommitRow>, BucketError>;
    async fn files_for_commit(&self, commit_id: Uuid) -> Result<Vec<CommitFileRow>, BucketError>;
}

#[async_trait]
impl CommitStore for DatabaseManager {
    async fn commits_for_bucket(&self, bucket_id: Uuid) -> Result<Vec<CommitRow>, BucketError> {
        let rows = self
            .query(
                "SELECT c.id, b.name, c.message, c.created_at, c.manifest
                 FROM commits c
                 JOIN buckets b ON c.bucket_id = b.id
                 WHERE c.bucket_id = $1
                 ORDER BY c.created_at ASC, c.id ASC",
                &[&bucket_id],
            )
            .await?;
        Ok(rows
            .iter()
            .map(|row| {
                let created_at: std::time::SystemTime = row.get(3);
                CommitRow {
                    id: row.get(0),
                    bucket: row.get(1),
                    message: row.get(2),
                    timestamp: chrono::DateTime::<chrono::Utc>::from(created_at).to_rfc3339(),
                    manifest: row.get(4),
                }
            })
            .collect())
    }

    async fn files_for_commit(&self, commit_id: Uuid) -> Result<Vec<CommitFileRow>, BucketError> {
        let rows = self
            .query(
                "SELECT f.id, f.file_path, f.hash FROM files f
                 WHERE f.commit_id = $1 ORDER BY f.file_path ASC",
                &[&commit_id],
            )
            .await?;
        Ok(rows
            .iter()
            .map(|row| CommitFileRow {
                id: row.get(0),
                file_path: row.get(1),
                hash: row.get(2),
            })
            .collect())
    }
}

const ZERO_HASH: [u8; 32] = [0u8; 32];

/// Lay one commit's rows over `files`. Commits made on Windows by older
/// versions stored `\\` separators; keys are normalised to `/` so they match
/// what the scanner reports now. If one commit holds both `a/b` and a
/// literal `a\\b` (possible on Linux before such names were refused), the
/// `/` spelling wins; the other file then shows up locally as a name that
/// commit and pull refuse until it is renamed, so nothing is overwritten.
fn overlay(files: &mut BTreeMap<String, CommitFileRow>, rows: Vec<CommitFileRow>) {
    let mut batch: BTreeMap<String, CommitFileRow> = BTreeMap::new();
    for mut row in rows {
        let native = !row.file_path.contains('\\');
        let key = row.file_path.replace('\\', "/");
        if batch.contains_key(&key) && !native {
            continue;
        }
        row.file_path = key.clone();
        batch.insert(key, row);
    }
    files.extend(batch);
}

/// Resolve a commit (the latest when `commit_id` is `None`) to its full file list.
pub async fn load_commit_with_store<S: CommitStore + ?Sized>(
    store: &S,
    bucket_id: Uuid,
    commit_id: Option<Uuid>,
) -> Result<Option<ResolvedCommit>, BucketError> {
    let commits = store.commits_for_bucket(bucket_id).await?;
    if commits.is_empty() {
        return match commit_id {
            Some(id) => Err(BucketError::NotFound(format!("Commit '{}' not found", id))),
            None => Ok(None),
        };
    }

    let target = match commit_id {
        Some(id) => commits.iter().position(|c| c.id == id).ok_or_else(|| {
            BucketError::NotFound(format!("Commit '{}' not found in this bucket", id))
        })?,
        None => commits.len() - 1,
    };

    // Start from the newest full manifest at or before the target.
    let start = commits[..=target].iter().rposition(|c| c.manifest);
    let mut files: BTreeMap<String, CommitFileRow> = BTreeMap::new();
    let first_overlay = match start {
        Some(index) => {
            overlay(&mut files, store.files_for_commit(commits[index].id).await?);
            index + 1
        }
        None => 0,
    };
    for commit in &commits[first_overlay..=target] {
        overlay(&mut files, store.files_for_commit(commit.id).await?);
    }

    let zero = Hash::from(ZERO_HASH);
    let mut committed = Vec::with_capacity(files.len());
    for (_, row) in files {
        let hash = Hash::from_hex(&row.hash).map_err(|e| {
            BucketError::from(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid hash '{}' for '{}': {}", row.hash, row.file_path, e),
            ))
        })?;
        committed.push(CommittedFile {
            id: row.id,
            name: row.file_path,
            hash,
            previous_hash: zero,
            status: CommitStatus::Committed,
        });
    }

    let commit = &commits[target];
    Ok(Some(ResolvedCommit {
        id: commit.id,
        data: CommitData {
            bucket: commit.bucket.clone(),
            files: committed,
            timestamp: commit.timestamp.clone(),
            previous: None,
            next: None,
        },
    }))
}

/// Resolve a commit using the global database.
pub async fn load_commit_async(
    bucket_id: Uuid,
    commit_id: Option<Uuid>,
) -> Result<Option<ResolvedCommit>, BucketError> {
    let db = get_database().await?;
    load_commit_with_store(&*db, bucket_id, commit_id).await
}

/// Every object hash referenced by any commit of the given buckets.
pub async fn referenced_hashes(
    db: &DatabaseManager,
    bucket_ids: &[Uuid],
) -> Result<Vec<String>, BucketError> {
    if bucket_ids.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Uuid> = bucket_ids.to_vec();
    let rows = db
        .query(
            "SELECT DISTINCT f.hash FROM files f
             JOIN commits c ON f.commit_id = c.id
             WHERE c.bucket_id = ANY($1)",
            &[&ids],
        )
        .await?;
    Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
}

/// Hashes this bucket has ever committed for each path (used to tell a clean
/// older version from a local edit).
pub async fn known_hashes_by_path(
    db: &DatabaseManager,
    bucket_id: Uuid,
) -> Result<BTreeMap<String, Vec<String>>, BucketError> {
    let rows = db
        .query(
            "SELECT DISTINCT f.file_path, f.hash FROM files f
             JOIN commits c ON f.commit_id = c.id
             WHERE c.bucket_id = $1",
            &[&bucket_id],
        )
        .await?;
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in rows {
        map.entry(row.get(0)).or_default().push(row.get(1));
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MockStore {
        commits: Vec<CommitRow>,
        files: HashMap<Uuid, Vec<CommitFileRow>>,
    }

    impl MockStore {
        fn add(&mut self, manifest: bool, files: &[(&str, &str)]) -> Uuid {
            let id = Uuid::new_v4();
            self.commits.push(CommitRow {
                id,
                bucket: "art".into(),
                message: format!("c{}", self.commits.len()),
                timestamp: chrono::Utc::now().to_rfc3339(),
                manifest,
            });
            self.files.insert(
                id,
                files
                    .iter()
                    .map(|(p, content)| CommitFileRow {
                        id: Uuid::new_v4(),
                        file_path: p.to_string(),
                        hash: blake3::hash(content.as_bytes()).to_hex().to_string(),
                    })
                    .collect(),
            );
            id
        }
    }

    #[async_trait]
    impl CommitStore for MockStore {
        async fn commits_for_bucket(&self, _: Uuid) -> Result<Vec<CommitRow>, BucketError> {
            Ok(self.commits.clone())
        }
        async fn files_for_commit(&self, id: Uuid) -> Result<Vec<CommitFileRow>, BucketError> {
            Ok(self.files.get(&id).cloned().unwrap_or_default())
        }
    }

    fn names(resolved: &ResolvedCommit) -> Vec<(String, Hash)> {
        resolved
            .data
            .files
            .iter()
            .map(|f| (f.name.clone(), f.hash))
            .collect()
    }

    #[tokio::test]
    async fn legacy_windows_separators_are_normalised() {
        let mut store = MockStore::default();
        store.add(true, &[("assets\\model.blend", "1")]);
        let resolved = load_commit_with_store(&store, Uuid::nil(), None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            names(&resolved),
            vec![("assets/model.blend".to_string(), blake3::hash(b"1"))]
        );

        // The `/` spelling wins over a literal backslash in the same commit.
        for rows in [[("a/b", "1"), ("a\\b", "2")], [("a\\b", "2"), ("a/b", "1")]] {
            let mut clash = MockStore::default();
            clash.add(true, &rows);
            let resolved = load_commit_with_store(&clash, Uuid::nil(), None)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                names(&resolved),
                vec![("a/b".to_string(), blake3::hash(b"1"))]
            );
        }
    }

    #[tokio::test]
    async fn empty_history_is_none() {
        let store = MockStore::default();
        assert!(load_commit_with_store(&store, Uuid::nil(), None)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn manifest_commit_is_read_as_is() {
        let mut store = MockStore::default();
        store.add(true, &[("a", "1"), ("b", "1"), ("c", "1")]);
        // c deleted, a edited: full manifest has only a and b.
        let last = store.add(true, &[("a", "2"), ("b", "1")]);
        let resolved = load_commit_with_store(&store, Uuid::nil(), None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(resolved.id, last);
        assert_eq!(
            names(&resolved),
            vec![
                ("a".to_string(), blake3::hash(b"2")),
                ("b".to_string(), blake3::hash(b"1"))
            ]
        );
    }

    #[tokio::test]
    async fn legacy_partial_commits_are_folded() {
        let mut store = MockStore::default();
        store.add(false, &[("a", "1"), ("b", "1"), ("c", "1")]);
        store.add(false, &[("a", "2")]);
        let resolved = load_commit_with_store(&store, Uuid::nil(), None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            names(&resolved),
            vec![
                ("a".to_string(), blake3::hash(b"2")),
                ("b".to_string(), blake3::hash(b"1")),
                ("c".to_string(), blake3::hash(b"1")),
            ]
        );
    }

    #[tokio::test]
    async fn older_commit_by_id_and_fold_after_manifest() {
        let mut store = MockStore::default();
        let first = store.add(false, &[("a", "1"), ("b", "1")]);
        store.add(true, &[("a", "1")]); // b deleted
        store.add(false, &[("a", "3")]); // hypothetical legacy writer afterwards

        let latest = load_commit_with_store(&store, Uuid::nil(), None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(names(&latest), vec![("a".to_string(), blake3::hash(b"3"))]);

        let old = load_commit_with_store(&store, Uuid::nil(), Some(first))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(old.data.files.len(), 2);

        assert!(
            load_commit_with_store(&store, Uuid::nil(), Some(Uuid::new_v4()))
                .await
                .is_err()
        );
    }
}
