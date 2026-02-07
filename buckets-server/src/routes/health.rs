// SPDX-License-Identifier: MIT

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

pub fn routes() -> Router {
    Router::new().route("/api/health", get(health))
}
