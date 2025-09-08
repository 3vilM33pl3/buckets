use crate::errors::BucketError;
use deadpool_postgres::{Config, Pool, Runtime};
use log::info;
use refinery::embed_migrations;
use std::time::Duration;
use tokio_postgres::NoTls;

// Embed migrations
embed_migrations!("src/sql/migrations");

/// Configuration for database connection
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: Option<String>,
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

        let (host_port, database) = rest
            .split_once('/')
            .ok_or_else(|| BucketError::from("Invalid PostgreSQL URL"))?;

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

/// Database connection manager
pub struct DatabaseManager {
    config: DatabaseConfig,
    pool: Option<Pool>,
}

impl DatabaseManager {
    /// Create a new database manager
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            config,
            pool: None,
        }
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

        info!("Connecting to external PostgreSQL database at {}:{}", self.config.host, self.config.port);

        // Create connection pool
        self.create_pool().await?;

        // Run migrations
        self.run_migrations().await?;

        Ok(())
    }

    /// Create connection pool
    async fn create_pool(&mut self) -> Result<(), BucketError> {
        let mut cfg = Config::new();
        cfg.host = Some(self.config.host.clone());
        cfg.port = Some(self.config.port);
        cfg.user = Some(self.config.username.clone());
        cfg.password = self.config.password.clone();
        cfg.dbname = Some(self.config.database.clone());
        cfg.connect_timeout = Some(Duration::from_secs(10));

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls).map_err(|e| {
            BucketError::from(format!("Failed to create connection pool: {}", e).as_str())
        })?;

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
static DATABASE: once_cell::sync::OnceCell<tokio::sync::Mutex<DatabaseManager>> =
    once_cell::sync::OnceCell::new();

/// Initialize the global database
pub async fn init_database(config: DatabaseConfig) -> Result<(), BucketError> {
    let mut manager = DatabaseManager::new(config);
    manager.initialize().await?;

    DATABASE
        .set(tokio::sync::Mutex::new(manager))
        .map_err(|_| BucketError::from("Database already initialized"))?;

    Ok(())
}

/// Get the global database manager
pub async fn get_database() -> Result<tokio::sync::MutexGuard<'static, DatabaseManager>, BucketError>
{
    let db = DATABASE
        .get()
        .ok_or_else(|| BucketError::from("Database not initialized"))?;
    Ok(db.lock().await)
}

/// Execute a database operation with the global database
#[allow(dead_code)] // Used for PostgreSQL migration
pub async fn with_database<F, T>(f: F) -> Result<T, BucketError>
where
    F: FnOnce(&DatabaseManager) -> T,
{
    let db = get_database().await?;
    Ok(f(&*db))
}
