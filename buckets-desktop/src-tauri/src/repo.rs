// SPDX-License-Identifier: MIT
//! Read-only views of a repository, built on the buckets library.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use buckets::data::bucket::read_bucket_info;
use buckets::data::lock::{current_owner, FileLock};
use buckets::data::manifest::load_commit_with_store;
use buckets::errors::BucketError;
use buckets::postgres_db::{get_database, DatabaseManager};
use buckets::utils::runtime::RuntimeManager;
use buckets::utils::scan::scan_bucket;
use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::model::{
    ActivityItem, Board, BucketCard, BucketFiles, ChangeCounts, CommitChange, CommitInfo,
    ExpectationInfo, FileEntry, LockInfo, PebbleInfo,
};

pub fn block_on<F, T>(future: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, BucketError>>,
{
    RuntimeManager::block_on(future).map_err(|e| e.to_string())
}

pub fn ts(value: NaiveDateTime) -> String {
    value.and_utc().to_rfc3339()
}

/// A bucket that exists both in the database and on disk in this repository.
/// (One database can serve several repositories.)
#[derive(Clone, Debug)]
pub struct RepoBucket {
    pub id: Uuid,
    pub name: String,
    pub rel: String,
    pub abs: PathBuf,
}

pub async fn repo_buckets(db: &DatabaseManager, root: &Path) -> Result<Vec<RepoBucket>, BucketError> {
    let rows = db
        .query("SELECT id, name, path FROM buckets ORDER BY created_at, name", &[])
        .await?;
    let mut out = Vec::new();
    for row in rows {
        let id: Uuid = row.get(0);
        let rel: String = row.get(2);
        let abs = root.join(&rel);
        match read_bucket_info(&abs) {
            Ok(info) if info.id == id => out.push(RepoBucket {
                id,
                name: row.get(1),
                rel,
                abs,
            }),
            _ => {}
        }
    }
    Ok(out)
}

pub fn find_bucket(root: &Path, bucket_id: Uuid) -> Result<RepoBucket, String> {
    block_on(async {
        let db = get_database().await?;
        let buckets = repo_buckets(&db, root).await?;
        buckets
            .into_iter()
            .find(|b| b.id == bucket_id)
            .ok_or_else(|| BucketError::NotFound(format!("Bucket {} is not in this repository", bucket_id)))
    })
}

fn lock_info(lock: FileLock, me: &str) -> LockInfo {
    LockInfo {
        bucket_id: lock.bucket_id,
        mine: lock.owner == me,
        path: lock.file_path,
        owner: lock.owner,
        created_at: ts(lock.created_at),
    }
}

/// Latest committed manifest of a bucket: path -> hash.
async fn latest_manifest(
    db: &DatabaseManager,
    bucket_id: Uuid,
) -> Result<(Option<Uuid>, BTreeMap<String, String>), BucketError> {
    let mut map = BTreeMap::new();
    let resolved = load_commit_with_store(db, bucket_id, None).await?;
    let id = resolved.as_ref().map(|r| r.id);
    if let Some(resolved) = resolved {
        for file in resolved.data.files {
            map.insert(file.name.replace('\\', "/"), file.hash.to_hex().to_string());
        }
    }
    Ok((id, map))
}

/// Working copy against the latest commit, plus locks.
fn compare_working(
    bucket: &RepoBucket,
    committed: &BTreeMap<String, String>,
    locks: &HashMap<String, LockInfo>,
) -> Result<Vec<FileEntry>, String> {
    let scanned = scan_bucket(&bucket.abs).map_err(|e| e.to_string())?;
    let mut files = Vec::with_capacity(scanned.len());
    let mut seen = std::collections::HashSet::new();
    for file in scanned {
        let path = file.path.replace('\\', "/");
        let hash = file.hash.to_hex().to_string();
        let committed_hash = committed.get(&path).cloned();
        let status = match &committed_hash {
            None => "new",
            Some(h) if *h == hash => "committed",
            Some(_) => "modified",
        };
        seen.insert(path.clone());
        files.push(FileEntry {
            lock: locks.get(&path).cloned(),
            path,
            status: status.to_string(),
            size: Some(file.size),
            hash: Some(hash),
            committed_hash,
        });
    }
    for (path, hash) in committed {
        if !seen.contains(path) {
            files.push(FileEntry {
                lock: locks.get(path).cloned(),
                path: path.clone(),
                status: "deleted".to_string(),
                size: None,
                hash: None,
                committed_hash: Some(hash.clone()),
            });
        }
    }
    files.sort_by_key(|f| f.path.to_lowercase());
    Ok(files)
}

fn counts(files: &[FileEntry]) -> ChangeCounts {
    let mut c = ChangeCounts::default();
    for f in files {
        match f.status.as_str() {
            "new" => c.added += 1,
            "modified" => c.modified += 1,
            "deleted" => c.deleted += 1,
            _ => {}
        }
    }
    c
}

async fn locks_for(
    db: &DatabaseManager,
    bucket_id: Uuid,
    me: &str,
) -> Result<HashMap<String, LockInfo>, BucketError> {
    Ok(FileLock::list_for_bucket(db, bucket_id)
        .await?
        .into_iter()
        .map(|l| (l.file_path.clone(), lock_info(l, me)))
        .collect())
}

pub fn bucket_files(root: &Path, bucket_id: Uuid) -> Result<BucketFiles, String> {
    let me = current_owner();
    let bucket = find_bucket(root, bucket_id)?;
    let (latest, committed, locks) = block_on(async {
        let db = get_database().await?;
        let (latest, committed) = latest_manifest(&db, bucket_id).await?;
        let locks = locks_for(&db, bucket_id, &me).await?;
        Ok((latest, committed, locks))
    })?;
    let files = compare_working(&bucket, &committed, &locks)?;
    Ok(BucketFiles {
        bucket_id,
        latest_commit: latest,
        files,
    })
}

/// One commit with the full set of files it recorded.
struct FoldedCommit {
    info: CommitInfo,
    changes: Vec<CommitChange>,
}

/// Walk a bucket's history oldest first, working out what each commit
/// changed. Full-manifest commits replace the file set; older commits that
/// only stored changed files are laid over the previous state.
async fn fold_history(db: &DatabaseManager, bucket_id: Uuid) -> Result<Vec<FoldedCommit>, BucketError> {
    let commits = db
        .query(
            "SELECT id, message, created_at, manifest FROM commits
             WHERE bucket_id = $1 ORDER BY created_at ASC, id ASC",
            &[&bucket_id],
        )
        .await?;
    let rows = db
        .query(
            "SELECT f.commit_id, f.file_path, f.hash FROM files f
             JOIN commits c ON c.id = f.commit_id WHERE c.bucket_id = $1",
            &[&bucket_id],
        )
        .await?;
    let mut by_commit: HashMap<Uuid, Vec<(String, String)>> = HashMap::new();
    for row in rows {
        let path: String = row.get(1);
        by_commit
            .entry(row.get(0))
            .or_default()
            .push((path.replace('\\', "/"), row.get(2)));
    }

    let mut state: BTreeMap<String, String> = BTreeMap::new();
    let mut out = Vec::with_capacity(commits.len());
    for commit in commits {
        let id: Uuid = commit.get(0);
        let manifest: bool = commit.get(3);
        let rows = by_commit.remove(&id).unwrap_or_default();
        let mut next = if manifest { BTreeMap::new() } else { state.clone() };
        for (path, hash) in rows {
            next.insert(path, hash);
        }
        let mut changes = Vec::new();
        for (path, hash) in &next {
            match state.get(path) {
                None => changes.push(CommitChange {
                    path: path.clone(),
                    status: "new".into(),
                    hash: Some(hash.clone()),
                    previous_hash: None,
                }),
                Some(old) if old != hash => changes.push(CommitChange {
                    path: path.clone(),
                    status: "modified".into(),
                    hash: Some(hash.clone()),
                    previous_hash: Some(old.clone()),
                }),
                _ => {}
            }
        }
        for (path, old) in &state {
            if !next.contains_key(path) {
                changes.push(CommitChange {
                    path: path.clone(),
                    status: "deleted".into(),
                    hash: None,
                    previous_hash: Some(old.clone()),
                });
            }
        }
        let count = |s: &str| changes.iter().filter(|c| c.status == s).count();
        let info = CommitInfo {
            id,
            bucket_id,
            message: commit.get(1),
            created_at: ts(commit.get(2)),
            added: count("new"),
            modified: count("modified"),
            deleted: count("deleted"),
            files: next.len(),
        };
        out.push(FoldedCommit { info, changes });
        state = next;
    }
    Ok(out)
}

pub fn history(bucket_id: Uuid) -> Result<Vec<CommitInfo>, String> {
    block_on(async {
        let db = get_database().await?;
        let mut commits: Vec<CommitInfo> = fold_history(&db, bucket_id)
            .await?
            .into_iter()
            .map(|c| c.info)
            .collect();
        commits.reverse();
        Ok(commits)
    })
}

pub fn commit_changes(bucket_id: Uuid, commit_id: Uuid) -> Result<Vec<CommitChange>, String> {
    block_on(async {
        let db = get_database().await?;
        fold_history(&db, bucket_id)
            .await?
            .into_iter()
            .find(|c| c.info.id == commit_id)
            .map(|c| c.changes)
            .ok_or_else(|| BucketError::NotFound(format!("Commit {}", commit_id)))
    })
}

pub fn board(root: &Path) -> Result<Board, String> {
    let me = current_owner();
    let (buckets, snapshot) = block_on(async {
        let db = get_database().await?;
        let buckets = repo_buckets(&db, root).await?;
        let ids: Vec<Uuid> = buckets.iter().map(|b| b.id).collect();

        let mut per_bucket = Vec::new();
        for bucket in &buckets {
            let count: i64 = db
                .query("SELECT COUNT(*) FROM commits WHERE bucket_id = $1", &[&bucket.id])
                .await?
                .first()
                .map(|r| r.get(0))
                .unwrap_or(0);
            let last = db
                .query(
                    "SELECT id, message, created_at FROM commits WHERE bucket_id = $1
                     ORDER BY created_at DESC, id DESC LIMIT 1",
                    &[&bucket.id],
                )
                .await?
                .first()
                .map(|r| CommitInfo {
                    id: r.get(0),
                    bucket_id: bucket.id,
                    message: r.get(1),
                    created_at: ts(r.get(2)),
                    added: 0,
                    modified: 0,
                    deleted: 0,
                    files: 0,
                });
            let (_, manifest) = latest_manifest(&db, bucket.id).await?;
            let locks = locks_for(&db, bucket.id, &me).await?;
            per_bucket.push((count, last, manifest, locks));
        }

        let pebbles: Vec<PebbleInfo> = db
            .query(
                "SELECT id, description, status, created_at, origin_bucket_id, current_bucket_id,
                        created_via_expectation_id
                 FROM pebbles WHERE current_bucket_id = ANY($1) ORDER BY created_at",
                &[&ids],
            )
            .await?
            .iter()
            .map(|r| PebbleInfo {
                id: r.get(0),
                description: r.get(1),
                status: r.get(2),
                created_at: ts(r.get(3)),
                origin_bucket_id: r.get(4),
                current_bucket_id: r.get(5),
                expectation_id: r.get(6),
            })
            .collect();

        let expectations: Vec<ExpectationInfo> = db
            .query(
                "SELECT id, description, status, created_at, bucket_id, target_bucket_id
                 FROM expectations WHERE bucket_id = ANY($1) ORDER BY created_at",
                &[&ids],
            )
            .await?
            .iter()
            .map(|r| {
                let id: Uuid = r.get(0);
                let status: String = r.get(2);
                let made: Vec<&PebbleInfo> = pebbles
                    .iter()
                    .filter(|p| p.expectation_id == Some(id))
                    .collect();
                let delivered = status == "met"
                    || (!made.is_empty() && made.iter().all(|p| p.status == "resolved"));
                ExpectationInfo {
                    id,
                    description: r.get(1),
                    status,
                    delivered,
                    created_at: ts(r.get(3)),
                    bucket_id: r.get(4),
                    from_bucket_id: r.get(5),
                }
            })
            .collect();
        Ok((buckets, (per_bucket, pebbles, expectations)))
    })?;

    let (per_bucket, pebbles, expectations) = snapshot;
    let mut cards = Vec::new();
    let mut all_locks = Vec::new();
    for (bucket, (commit_count, last_commit, manifest, locks)) in buckets.iter().zip(per_bucket) {
        let (files, scan_error) = match compare_working(bucket, &manifest, &locks) {
            Ok(files) => (files, None),
            Err(e) => (Vec::new(), Some(e)),
        };
        all_locks.extend(locks.into_values());
        cards.push(BucketCard {
            id: bucket.id,
            name: bucket.name.clone(),
            path: bucket.rel.clone(),
            commit_count,
            file_count: files.iter().filter(|f| f.status != "deleted").count(),
            changes: counts(&files),
            last_commit,
            scan_error,
        });
    }
    all_locks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(Board {
        buckets: cards,
        expectations,
        pebbles,
        locks: all_locks,
    })
}

pub fn activity(root: &Path, limit: i64) -> Result<Vec<ActivityItem>, String> {
    block_on(async {
        let db = get_database().await?;
        let buckets = repo_buckets(&db, root).await?;
        let names: HashMap<Uuid, String> = buckets.iter().map(|b| (b.id, b.name.clone())).collect();
        let ids: Vec<Uuid> = buckets.iter().map(|b| b.id).collect();
        let rows = db
            .query(
                "SELECT c.id, c.bucket_id, c.message, c.created_at,
                        (SELECT COUNT(*) FROM files f WHERE f.commit_id = c.id)
                 FROM commits c WHERE c.bucket_id = ANY($1)
                 ORDER BY c.created_at DESC, c.id DESC LIMIT $2",
                &[&ids, &limit],
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let bucket_id: Uuid = r.get(1);
                let files: i64 = r.get(4);
                ActivityItem {
                    bucket_name: names.get(&bucket_id).cloned().unwrap_or_default(),
                    commit: CommitInfo {
                        id: r.get(0),
                        bucket_id,
                        message: r.get(2),
                        created_at: ts(r.get(3)),
                        added: 0,
                        modified: 0,
                        deleted: 0,
                        files: files.max(0) as usize,
                    },
                }
            })
            .collect())
    })
}
