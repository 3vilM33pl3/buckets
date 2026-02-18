use crate::errors::BucketError;
use deadpool_postgres::{Config, Pool, Runtime};
use log::info;
use once_cell::sync::Lazy;
use refinery::embed_migrations;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::{Mutex, MutexGuard};
use tokio_postgres::NoTls;

// Embed migrations
embed_migrations!("src/sql/migrations");

/// TLS configuration for PostgreSQL connections
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TlsConfig {
    /// Path to the CA certificate PEM file (enables server-verified TLS)
    pub ca_cert: Option<String>,
    /// Path to the client certificate PEM file (enables mTLS when combined with client_key)
    pub client_cert: Option<String>,
    /// Path to the client private key PEM file
    pub client_key: Option<String>,
}

impl TlsConfig {
    /// Returns true if any TLS fields are configured
    pub fn is_enabled(&self) -> bool {
        self.ca_cert.is_some()
    }

    /// Returns true if full mutual TLS is configured (CA + client cert + client key)
    pub fn is_mtls(&self) -> bool {
        self.ca_cert.is_some() && self.client_cert.is_some() && self.client_key.is_some()
    }
}

/// Configuration for database connection
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: Option<String>,
    pub tls: Option<TlsConfig>,
}

impl DatabaseConfig {
    /// Create config from environment or defaults
    #[allow(dead_code)]
    pub fn from_env() -> Result<Self, BucketError> {
        if let Ok(url) = std::env::var("DATABASE_URL") {
            // Parse PostgreSQL connection URL
            Self::from_url(&url)
        } else {
            Err(BucketError::from("No DATABASE_URL environment variable found. External database configuration is required."))
        }
    }

    /// Parse a PostgreSQL connection URL
    #[allow(dead_code)] // Used for PostgreSQL migration
    pub fn from_url(url: &str) -> Result<Self, BucketError> {
        // Parse postgresql://username:password@host:port/database
        let url = url
            .strip_prefix("postgresql://")
            .or_else(|| url.strip_prefix("postgres://"))
            .ok_or_else(|| BucketError::from("Invalid PostgreSQL URL"))?;

        let (auth, rest) = url
            .split_once('@')
            .ok_or_else(|| BucketError::from("Invalid PostgreSQL URL"))?;

        let (username, password) = if let Some((u, p)) = auth.split_once(':') {
            (u.to_string(), Some(p.to_string()))
        } else {
            (auth.to_string(), None)
        };

        let (host_port, db_and_params) = rest
            .split_once('/')
            .ok_or_else(|| BucketError::from("Invalid PostgreSQL URL"))?;

        // Strip query parameters (e.g. ?sslmode=disable)
        let database = db_and_params
            .split_once('?')
            .map_or(db_and_params, |(db, _)| db);

        let (host, port) = if let Some((h, p)) = host_port.split_once(':') {
            (h.to_string(), p.parse().unwrap_or(5432))
        } else {
            (host_port.to_string(), 5432)
        };

        Ok(Self {
            host,
            port,
            database: database.to_string(),
            username,
            password,
            tls: None,
        })
    }

    /// Get connection string for this configuration
    #[allow(dead_code)] // Used for PostgreSQL migration
    pub fn connection_string(&self) -> String {
        let mut conn = format!(
            "host={} port={} user={} dbname={}",
            self.host, self.port, self.username, self.database
        );
        if let Some(pwd) = &self.password {
            conn.push_str(&format!(" password={}", pwd));
        }
        conn
    }
}

/// Build a rustls ClientConfig from TLS certificate paths
fn build_rustls_config(tls: &TlsConfig) -> Result<rustls::ClientConfig, BucketError> {
    // Install the ring crypto provider (idempotent — ignores AlreadyInstalled error)
    let _ = rustls::crypto::ring::default_provider().install_default();

    let ca_path = tls
        .ca_cert
        .as_ref()
        .ok_or_else(|| BucketError::from("TLS enabled but ca_cert path is missing"))?;

    // Load CA certificates
    let ca_pem = std::fs::read(ca_path).map_err(|e| {
        BucketError::from(format!("Failed to read CA certificate '{}': {}", ca_path, e).as_str())
    })?;
    let mut root_store = rustls::RootCertStore::empty();
    let ca_certs = rustls_pemfile::certs(&mut &ca_pem[..])
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            BucketError::from(format!("Failed to parse CA certificate: {}", e).as_str())
        })?;
    for cert in ca_certs {
        root_store.add(cert).map_err(|e| {
            BucketError::from(format!("Failed to add CA certificate to root store: {}", e).as_str())
        })?;
    }

    let config = if tls.is_mtls() {
        let cert_path = tls
            .client_cert
            .as_ref()
            .ok_or_else(|| BucketError::from("mTLS enabled but client_cert path is missing"))?;
        let key_path = tls
            .client_key
            .as_ref()
            .ok_or_else(|| BucketError::from("mTLS enabled but client_key path is missing"))?;

        // Load client certificate chain
        let cert_pem = std::fs::read(cert_path).map_err(|e| {
            BucketError::from(
                format!("Failed to read client certificate '{}': {}", cert_path, e).as_str(),
            )
        })?;
        let client_certs = rustls_pemfile::certs(&mut &cert_pem[..])
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                BucketError::from(format!("Failed to parse client certificate: {}", e).as_str())
            })?;

        // Load client private key
        let key_pem = std::fs::read(key_path).map_err(|e| {
            BucketError::from(format!("Failed to read client key '{}': {}", key_path, e).as_str())
        })?;
        let client_key = rustls_pemfile::private_key(&mut &key_pem[..])
            .map_err(|e| BucketError::from(format!("Failed to parse client key: {}", e).as_str()))?
            .ok_or_else(|| BucketError::from("No private key found in client key file"))?;

        rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(client_certs, client_key)
            .map_err(|e| {
                BucketError::from(format!("Failed to build mTLS client config: {}", e).as_str())
            })?
    } else {
        rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth()
    };

    Ok(config)
}

/// Create a connection pool from a DatabaseConfig, using TLS if configured
pub fn create_connection_pool(db_config: &DatabaseConfig) -> Result<Pool, BucketError> {
    let mut cfg = Config::new();
    cfg.host = Some(db_config.host.clone());
    cfg.port = Some(db_config.port);
    cfg.user = Some(db_config.username.clone());
    cfg.password = db_config.password.clone();
    cfg.dbname = Some(db_config.database.clone());
    cfg.connect_timeout = Some(Duration::from_secs(10));

    let tls_enabled = db_config.tls.as_ref().is_some_and(|t| t.is_enabled());

    if tls_enabled {
        let tls = db_config
            .tls
            .as_ref()
            .ok_or_else(|| BucketError::from("TLS config disappeared unexpectedly"))?;
        let rustls_config = build_rustls_config(tls)?;
        let tls_connector = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);
        cfg.create_pool(Some(Runtime::Tokio1), tls_connector)
            .map_err(|e| {
                BucketError::from(format!("Failed to create TLS connection pool: {}", e).as_str())
            })
    } else {
        cfg.create_pool(Some(Runtime::Tokio1), NoTls).map_err(|e| {
            BucketError::from(format!("Failed to create connection pool: {}", e).as_str())
        })
    }
}

/// Database connection manager
pub struct DatabaseManager {
    config: DatabaseConfig,
    pool: Option<Pool>,
}

impl DatabaseManager {
    /// Create a new database manager
    pub fn new(config: DatabaseConfig) -> Self {
        Self { config, pool: None }
    }

    /// Initialize the database (create pool, run migrations)
    pub async fn initialize(&mut self) -> Result<(), BucketError> {
        // Check if we're in a test environment and should skip database initialization
        if std::env::var("BUCKETS_SKIP_DB_INIT").is_ok() {
            info!(
                "Skipping database initialization due to BUCKETS_SKIP_DB_INIT environment variable"
            );
            return Ok(());
        }

        info!(
            "Connecting to external PostgreSQL database at {}:{}",
            self.config.host, self.config.port
        );

        // Create connection pool
        self.create_pool().await?;

        // Run migrations
        self.run_migrations().await?;

        Ok(())
    }

    /// Create connection pool
    async fn create_pool(&mut self) -> Result<(), BucketError> {
        let pool = create_connection_pool(&self.config)?;

        // Test connection
        let _ = pool.get().await.map_err(|e| {
            BucketError::from(format!("Failed to connect to database: {}", e).as_str())
        })?;

        self.pool = Some(pool);
        Ok(())
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<(), BucketError> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| BucketError::from("No connection pool available"))?;

        let mut client = pool.get().await.map_err(|e| {
            BucketError::from(format!("Failed to get database connection: {}", e).as_str())
        })?;

        let runner = migrations::runner();

        // Run migrations
        runner
            .run_async(&mut **client)
            .await
            .map_err(|e| BucketError::from(format!("Failed to run migrations: {}", e).as_str()))?;

        info!("Database migrations completed");
        Ok(())
    }

    /// Get a database connection from the pool
    pub async fn get_connection(&self) -> Result<deadpool_postgres::Object, BucketError> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| BucketError::from("Database not initialized"))?;

        pool.get().await.map_err(|e| {
            BucketError::from(format!("Failed to get database connection: {}", e).as_str())
        })
    }

    /// Execute a query that returns no results
    pub async fn execute(
        &self,
        query: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<u64, BucketError> {
        let client = self.get_connection().await?;
        client
            .execute(query, params)
            .await
            .map_err(|e| BucketError::from(format!("Query failed: {}", e).as_str()))
    }

    /// Execute a query that returns results
    pub async fn query(
        &self,
        query: &str,
        params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
    ) -> Result<Vec<tokio_postgres::Row>, BucketError> {
        let client = self.get_connection().await?;
        client
            .query(query, params)
            .await
            .map_err(|e| BucketError::from(format!("Query failed: {}", e).as_str()))
    }
}

// Global database manager instance
static DATABASE: Lazy<Mutex<Option<DatabaseManager>>> = Lazy::new(|| Mutex::new(None));

/// Initialize the global database
pub async fn init_database(config: DatabaseConfig) -> Result<(), BucketError> {
    let mut manager = DatabaseManager::new(config);
    manager.initialize().await?;

    let mut slot = DATABASE.lock().await;
    if slot.is_some() {
        return Err(BucketError::DatabaseAlreadyInitialized);
    }
    *slot = Some(manager);

    Ok(())
}

/// Get the global database manager
pub async fn get_database() -> Result<DatabaseHandle<'static>, BucketError> {
    if let Some(handle) = try_get_database().await {
        return Ok(handle);
    }

    initialize_database_from_env().await?;

    if let Some(handle) = try_get_database().await {
        return Ok(handle);
    }

    Err(BucketError::from("Database not initialized"))
}

/// Execute a database operation with the global database
#[allow(dead_code)] // Used for PostgreSQL migration
pub async fn with_database<F, T>(f: F) -> Result<T, BucketError>
where
    F: FnOnce(&DatabaseManager) -> T,
{
    let db = get_database().await?;
    Ok(f(&db))
}

pub struct DatabaseHandle<'a> {
    guard: MutexGuard<'a, Option<DatabaseManager>>,
}

impl<'a> std::ops::Deref for DatabaseHandle<'a> {
    type Target = DatabaseManager;

    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().expect("database initialized")
    }
}

impl<'a> std::ops::DerefMut for DatabaseHandle<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.as_mut().expect("database initialized")
    }
}

#[cfg(test)]
pub async fn reset_database_for_tests() {
    let mut guard = DATABASE.lock().await;
    if let Some(manager) = guard.take() {
        drop(manager);
    }
}

async fn try_get_database() -> Option<DatabaseHandle<'static>> {
    let guard = DATABASE.lock().await;
    if guard.is_some() {
        Some(DatabaseHandle { guard })
    } else {
        None
    }
}

async fn initialize_database_from_env() -> Result<(), BucketError> {
    let config = DatabaseConfig::from_env()?;
    match init_database(config).await {
        Ok(_) => Ok(()),
        Err(err) => match err {
            BucketError::DatabaseAlreadyInitialized => Ok(()),
            other => Err(other),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_url_basic() {
        let config =
            DatabaseConfig::from_url("postgresql://user:pass@localhost:5432/mydb").unwrap();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.username, "user");
        assert_eq!(config.password, Some("pass".to_string()));
        assert_eq!(config.database, "mydb");
    }

    #[test]
    fn test_from_url_strips_query_params() {
        let config = DatabaseConfig::from_url(
            "postgresql://olivier:secret@10.22.6.42:5432/buckets?sslmode=disable",
        )
        .unwrap();
        assert_eq!(config.host, "10.22.6.42");
        assert_eq!(config.port, 5432);
        assert_eq!(config.username, "olivier");
        assert_eq!(config.database, "buckets");
    }

    #[test]
    fn test_from_url_multiple_query_params() {
        let config = DatabaseConfig::from_url(
            "postgresql://u:p@host:5432/db?sslmode=disable&connect_timeout=10",
        )
        .unwrap();
        assert_eq!(config.database, "db");
    }

    #[test]
    fn test_from_url_no_query_params() {
        let config = DatabaseConfig::from_url("postgresql://u:p@host:5432/db").unwrap();
        assert_eq!(config.database, "db");
    }

    #[test]
    fn test_from_url_postgres_scheme() {
        let config = DatabaseConfig::from_url("postgres://u:p@host:5432/db").unwrap();
        assert_eq!(config.database, "db");
    }
}
