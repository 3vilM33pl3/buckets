// SPDX-License-Identifier: MIT

use axum::extract::Query;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::db;
use crate::error::ApiError;
use crate::models::SemanticResult;

#[derive(Deserialize)]
struct SearchParams {
    q: String,
}

async fn search(
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<SemanticResult>>, ApiError> {
    if params.q.is_empty() {
        return Err(ApiError::BadRequest("Query parameter 'q' cannot be empty".to_string()));
    }
    let results = db::semantic_search(&params.q).await?;
    Ok(Json(results))
}

pub fn routes() -> Router {
    Router::new().route("/api/search", get(search))
}
