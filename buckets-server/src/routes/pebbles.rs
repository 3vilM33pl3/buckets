// SPDX-License-Identifier: MIT

use axum::extract::Path;
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use crate::db;
use crate::error::ApiError;
use crate::models::PebbleRow;

async fn list_pebbles() -> Result<Json<Vec<PebbleRow>>, ApiError> {
    let pebbles = db::fetch_pebbles().await?;
    Ok(Json(pebbles))
}

async fn list_pebbles_for_bucket(Path(id): Path<Uuid>) -> Result<Json<Vec<PebbleRow>>, ApiError> {
    let pebbles = db::fetch_pebbles_for_bucket(id).await?;
    Ok(Json(pebbles))
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/pebbles", get(list_pebbles))
        .route("/api/buckets/{id}/pebbles", get(list_pebbles_for_bucket))
}
