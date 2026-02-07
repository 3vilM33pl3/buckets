// SPDX-License-Identifier: MIT

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketRow {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub created_at: NaiveDateTime,
    pub commit_count: i64,
    pub expectation_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRow {
    pub id: Uuid,
    pub bucket_name: String,
    pub message: String,
    pub file_count: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRow {
    pub id: Uuid,
    pub file_path: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectationRow {
    pub id: Uuid,
    pub description: String,
    pub status: String,
    pub source_bucket: String,
    pub target_bucket: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PebbleRow {
    pub id: Uuid,
    pub description: String,
    pub origin_bucket: String,
    pub current_bucket: String,
    pub status: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticResult {
    pub expectation: ExpectationRow,
    pub similarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: Uuid,
    pub to: Uuid,
    pub count: i64,
}
