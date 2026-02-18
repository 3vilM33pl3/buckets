// SPDX-License-Identifier: MIT

mod config;
mod db;
mod error;
mod models;
mod routes;

use buckets::postgres_db::{init_database, DatabaseConfig, TlsConfig};
use clap::Parser;
use config::ServerArgs;
use log::info;
use tokio::net::TcpListener;

const ENV_DATABASE_URL: &str = "DATABASE_URL";
const ENV_TLS_CA_CERT: &str = "BUCKETS_DB_TLS_CA_CERT_PATH";
const ENV_TLS_CLIENT_CERT: &str = "BUCKETS_DB_TLS_CLIENT_CERT_PATH";
const ENV_TLS_CLIENT_KEY: &str = "BUCKETS_DB_TLS_CLIENT_KEY_PATH";

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn build_tls_config(
    ca_cert: Option<String>,
    client_cert: Option<String>,
    client_key: Option<String>,
) -> Result<Option<TlsConfig>, String> {
    if ca_cert.is_none() && client_cert.is_none() && client_key.is_none() {
        return Ok(None);
    }

    if ca_cert.is_none() {
        return Err(format!(
            "{} is required when TLS env vars are set",
            ENV_TLS_CA_CERT
        ));
    }

    let cert_set = client_cert.is_some();
    let key_set = client_key.is_some();
    if cert_set ^ key_set {
        return Err(format!(
            "{} and {} must both be set for mTLS client authentication",
            ENV_TLS_CLIENT_CERT, ENV_TLS_CLIENT_KEY
        ));
    }

    Ok(Some(TlsConfig {
        ca_cert,
        client_cert,
        client_key,
    }))
}

fn database_config_from_env() -> Result<DatabaseConfig, Box<dyn std::error::Error>> {
    let database_url = std::env::var(ENV_DATABASE_URL).map_err(|_| {
        format!(
            "{} environment variable is required. Example: {}=postgresql://user:pass@host:5432/dbname",
            ENV_DATABASE_URL, ENV_DATABASE_URL
        )
    })?;

    let mut db_config = DatabaseConfig::from_url(&database_url)?;
    db_config.tls = build_tls_config(
        optional_env(ENV_TLS_CA_CERT),
        optional_env(ENV_TLS_CLIENT_CERT),
        optional_env(ENV_TLS_CLIENT_KEY),
    )
    .map_err(|e| format!("Invalid TLS environment configuration: {}", e))?;

    Ok(db_config)
}

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

    let db_config = database_config_from_env()?;
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

#[cfg(test)]
mod tests {
    use super::build_tls_config;

    #[test]
    fn tls_config_disabled_when_no_values() {
        let result = build_tls_config(None, None, None);
        assert!(result.is_ok());
        assert!(result.ok().flatten().is_none());
    }

    #[test]
    fn tls_config_rejects_missing_ca() {
        let result = build_tls_config(None, Some("client.crt".to_string()), Some("client.key".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn tls_config_rejects_partial_client_pair() {
        let only_cert = build_tls_config(
            Some("ca.crt".to_string()),
            Some("client.crt".to_string()),
            None,
        );
        assert!(only_cert.is_err());

        let only_key = build_tls_config(
            Some("ca.crt".to_string()),
            None,
            Some("client.key".to_string()),
        );
        assert!(only_key.is_err());
    }

    #[test]
    fn tls_config_allows_server_only_tls() {
        let result = build_tls_config(Some("ca.crt".to_string()), None, None);
        assert!(result.is_ok());
        let tls = result.ok().flatten();
        assert!(tls.is_some());
        let tls = tls.unwrap_or_default();
        assert!(tls.is_enabled());
        assert!(!tls.is_mtls());
    }

    #[test]
    fn tls_config_allows_mtls() {
        let result = build_tls_config(
            Some("ca.crt".to_string()),
            Some("client.crt".to_string()),
            Some("client.key".to_string()),
        );
        assert!(result.is_ok());
        let tls = result.ok().flatten().unwrap_or_default();
        assert!(tls.is_enabled());
        assert!(tls.is_mtls());
    }
}
