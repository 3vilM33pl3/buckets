// SPDX-License-Identifier: MIT

use axum::extract::Path;
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use crate::db;
use crate::error::ApiError;
use crate::models::{CommitRow, FileRow};

async fn list_commits_for_bucket(
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<CommitRow>>, ApiError> {
    let commits = db::fetch_commits(id).await?;
    Ok(Json(commits))
}

async fn list_files_for_commit(
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<FileRow>>, ApiError> {
    let files = db::fetch_files(id).await?;
    Ok(Json(files))
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/buckets/{id}/commits", get(list_commits_for_bucket))
        .route("/api/commits/{id}/files", get(list_files_for_commit))
}
