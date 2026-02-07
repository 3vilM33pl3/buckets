// SPDX-License-Identifier: MIT

pub mod buckets;
pub mod commits;
pub mod expectations;
pub mod graph;
pub mod health;
pub mod pebbles;
pub mod search;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};

pub fn build_router() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET])
        .allow_headers(Any);

    Router::new()
        .merge(health::routes())
        .merge(buckets::routes())
        .merge(commits::routes())
        .merge(expectations::routes())
        .merge(pebbles::routes())
        .merge(graph::routes())
        .merge(search::routes())
        .layer(cors)
}
