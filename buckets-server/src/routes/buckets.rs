// SPDX-License-Identifier: MIT

use axum::extract::Path;
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use crate::db;
use crate::error::ApiError;
use crate::models::BucketRow;

async fn list_buckets() -> Result<Json<Vec<BucketRow>>, ApiError> {
    let buckets = db::fetch_buckets().await?;
    Ok(Json(buckets))
}

async fn get_bucket(Path(id): Path<Uuid>) -> Result<Json<BucketRow>, ApiError> {
    let bucket = db::fetch_bucket_by_id(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Bucket {id} not found")))?;
    Ok(Json(bucket))
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/buckets", get(list_buckets))
        .route("/api/buckets/{id}", get(get_bucket))
}
