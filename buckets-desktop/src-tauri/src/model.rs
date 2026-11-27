// SPDX-License-Identifier: MIT
//! Data sent to the web view. Field names are camelCase on the JS side.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub name: String,
    pub root: String,
    pub remote: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CliInfo {
    pub path: Option<String>,
    pub version: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub repo: Option<RepoInfo>,
    pub error: Option<String>,
    pub me: String,
    pub recent: Vec<String>,
    pub cli: CliInfo,
    pub settings: crate::session::Settings,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    pub id: Uuid,
    pub bucket_id: Uuid,
    pub message: String,
    pub created_at: String,
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
    pub files: usize,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCounts {
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BucketCard {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub commit_count: i64,
    pub file_count: usize,
    pub last_commit: Option<CommitInfo>,
    pub changes: ChangeCounts,
    pub scan_error: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExpectationInfo {
    pub id: Uuid,
    pub description: String,
    /// `pending` or `met`, as stored.
    pub status: String,
    /// True when stored as met, or every pebble it created has been resolved
    /// (finalize only marks intrinsic expectations as met).
    pub delivered: bool,
    pub created_at: String,
    /// The bucket that holds (is waiting on) the expectation.
    pub bucket_id: Uuid,
    /// The bucket expected to deliver it, if any.
    pub from_bucket_id: Option<Uuid>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PebbleInfo {
    pub id: Uuid,
    pub description: String,
    pub status: String,
    pub created_at: String,
    pub origin_bucket_id: Uuid,
    pub current_bucket_id: Uuid,
    pub expectation_id: Option<Uuid>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LockInfo {
    pub bucket_id: Uuid,
    pub path: String,
    pub owner: String,
    pub created_at: String,
    pub mine: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub buckets: Vec<BucketCard>,
    pub expectations: Vec<ExpectationInfo>,
    pub pebbles: Vec<PebbleInfo>,
    pub locks: Vec<LockInfo>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    /// `committed`, `new`, `modified` or `deleted`.
    pub status: String,
    pub size: Option<u64>,
    /// Hash of the working copy (None when deleted).
    pub hash: Option<String>,
    /// Hash in the latest commit (None when new).
    pub committed_hash: Option<String>,
    pub lock: Option<LockInfo>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BucketFiles {
    pub bucket_id: Uuid,
    pub latest_commit: Option<Uuid>,
    pub files: Vec<FileEntry>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitChange {
    pub path: String,
    pub status: String,
    pub hash: Option<String>,
    pub previous_hash: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActivityItem {
    pub commit: CommitInfo,
    pub bucket_name: String,
}

/// Which version of a file to show.
#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Source {
    /// The file as it is on disk now.
    Working,
    /// A committed object.
    Object { hash: String },
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    /// `image`, `text`, `binary` or `missing`.
    pub kind: String,
    pub size: Option<u64>,
    pub hash: Option<String>,
    pub data_url: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,
    /// A note on how the preview was made (e.g. "HDR clamped to 0–1").
    pub note: Option<String>,
    pub text: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImageDiff {
    pub data_url: String,
    pub changed_percent: f64,
    pub mean_delta: f64,
    pub max_delta: u8,
    pub same_size: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    /// `equal`, `insert` or `delete`.
    pub tag: String,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub text: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub left: Preview,
    pub right: Preview,
    pub image: Option<ImageDiff>,
    pub text: Option<Vec<DiffLine>>,
}
