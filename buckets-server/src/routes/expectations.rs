// SPDX-License-Identifier: MIT

use axum::extract::Path;
use axum::routing::get;
use axum::{Json, Router};
use uuid::Uuid;

use crate::db;
use crate::error::ApiError;
use crate::models::ExpectationRow;

async fn list_expectations() -> Result<Json<Vec<ExpectationRow>>, ApiError> {
    let expectations = db::fetch_expectations().await?;
    Ok(Json(expectations))
}

async fn list_expectations_for_bucket(
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ExpectationRow>>, ApiError> {
    let expectations = db::fetch_expectations_for_bucket(id).await?;
    Ok(Json(expectations))
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/expectations", get(list_expectations))
        .route(
            "/api/buckets/{id}/expectations",
            get(list_expectations_for_bucket),
        )
}
