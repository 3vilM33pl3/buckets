// SPDX-License-Identifier: MIT

use axum::routing::get;
use axum::{Json, Router};

use crate::db;
use crate::error::ApiError;
use crate::models::GraphData;

async fn get_graph() -> Result<Json<GraphData>, ApiError> {
    let graph = db::fetch_graph_data().await?;
    Ok(Json(graph))
}

pub fn routes() -> Router {
    Router::new().route("/api/graph", get(get_graph))
}
