// SPDX-License-Identifier: MIT

mod config;
mod db;
mod error;
mod models;
mod routes;

use buckets::postgres_db::{DatabaseConfig, init_database};
use clap::Parser;
use config::ServerArgs;
use log::info;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = ServerArgs::parse();

    env_logger::Builder::new()
        .filter_level(if args.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        "DATABASE_URL environment variable is required. Example: DATABASE_URL=postgresql://user:pass@host:5432/dbname"
    })?;

    let db_config = DatabaseConfig::from_url(&database_url)?;
    init_database(db_config).await?;
    info!("Database initialized");

    let router = routes::build_router();
    let addr = format!("{}:{}", args.host, args.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("Server listening on {addr}");

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    info!("Shutting down");
}
