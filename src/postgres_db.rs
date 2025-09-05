use crate::errors::BucketError;
use deadpool_postgres::{Config, Pool, Runtime};
use log::info;
use postgresql_embedded::{PostgreSQL, Settings, VersionReq};
use refinery::embed_migrations;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_postgres::NoTls;

// Embed migrations
embed_migrations!("src/sql/migrations");

/// Configuration for database connection
#[derive(Debug, Clone)]
pub enum DatabaseConfig {
    /// Use an embedded PostgreSQL instance
    Embedded {
        data_dir: PathBuf,
        port: Option<u16>,
    },
    /// Connect to an external PostgreSQL server
    #[allow(dead_code)] // Used for PostgreSQL migration
    External {
        host: String,
        port: u16,
        database: String,
        username: String,
        password: Option<String>,
    },
}

impl DatabaseConfig {
    /// Create config from environment or defaults
    #[allow(dead_code)] // Used for PostgreSQL migration
    pub fn from_env(repo_path: &Path) -> Self {
        if let Ok(url) = std::env::var("DATABASE_URL") {
            // Parse PostgreSQL connection URL
            Self::from_url(&url).unwrap_or_else(|_| Self::Embedded {
                data_dir: repo_path.join(".b").join("postgres"),
                port: None,
            })
        } else {
            Self::Embedded {
                data_dir: repo_path.join(".b").join("postgres"),
                port: None,
            }
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

        Ok(Self::External {
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
        match self {
            Self::Embedded { port, .. } => {
                let port = port.unwrap_or(5432);
                format!("host=localhost port={} user=postgres dbname=buckets", port)
            }
            Self::External {
                host,
                port,
                database,
                username,
                password,
            } => {
                let mut conn = format!(
                    "host={} port={} user={} dbname={}",
                    host, port, username, database
                );
                if let Some(pwd) = password {
                    conn.push_str(&format!(" password={}", pwd));
                }
                conn
            }
        }
    }
}

/// Database connection manager
pub struct DatabaseManager {
    config: DatabaseConfig,
    embedded_pg: Option<PostgreSQL>,
    pool: Option<Pool>,
}

impl DatabaseManager {
    /// Create a new database manager
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            config,
            embedded_pg: None,
            pool: None,
        }
    }

    /// Initialize the database (start embedded if needed, run migrations)
    pub async fn initialize(&mut self) -> Result<(), BucketError> {
        // Check if we're in a test environment and should skip database initialization
        if std::env::var("BUCKETS_SKIP_DB_INIT").is_ok() {
            info!(
                "Skipping database initialization due to BUCKETS_SKIP_DB_INIT environment variable"
            );
            return Ok(());
        }

        // Start embedded PostgreSQL if needed
        if let DatabaseConfig::Embedded { data_dir, port } = &self.config {
            info!("Starting embedded PostgreSQL...");

            // Create data directory if it doesn't exist
            std::fs::create_dir_all(data_dir).map_err(|e| {
                BucketError::from(format!("Failed to create data directory: {}", e).as_str())
            })?;

            let mut settings = Settings::default();
            settings.version = VersionReq::parse(">=13.0.0").map_err(|e| {
                BucketError::from(format!("Invalid version requirement: {}", e).as_str())
            })?;
            settings.installation_dir = data_dir.join("install");
            settings.data_dir = data_dir.join("data");
            settings.port = port.unwrap_or(0); // 0 means auto-select port
            settings.temporary = false;
            settings.password_file = data_dir.join(".pgpass");

            let mut pg = PostgreSQL::new(settings);

            // Install PostgreSQL if not already installed
            if !pg.settings().installation_dir.exists() {
                info!("Installing PostgreSQL...");
                pg.setup().await.map_err(|e| {
                    BucketError::from(format!("Failed to install PostgreSQL: {}", e).as_str())
                })?;
            }

            // Start PostgreSQL
            pg.start().await.map_err(|e| {
                BucketError::from(format!("Failed to start PostgreSQL: {}", e).as_str())
            })?;

            // Update config with actual port
            if let DatabaseConfig::Embedded { port: cfg_port, .. } = &mut self.config {
                *cfg_port = Some(pg.settings().port);
            }

            info!("PostgreSQL started on port {}", pg.settings().port);
            self.embedded_pg = Some(pg);
        }

        // Create connection pool
        self.create_pool().await?;

        // Run migrations
        self.run_migrations().await?;

        Ok(())
    }

    /// Create connection pool
    async fn create_pool(&mut self) -> Result<(), BucketError> {
        let mut cfg = Config::new();

        match &self.config {
            DatabaseConfig::Embedded { port, .. } => {
                cfg.host = Some("localhost".to_string());
                cfg.port = Some(port.unwrap_or(5432));
                cfg.user = Some("postgres".to_string());
                cfg.dbname = Some("buckets".to_string());
            }
            DatabaseConfig::External {
                host,
                port,
                database,
                username,
                password,
            } => {
                cfg.host = Some(host.clone());
                cfg.port = Some(*port);
                cfg.user = Some(username.clone());
                cfg.password = password.clone();
                cfg.dbname = Some(database.clone());
            }
        }

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

    /// Shutdown the database (for embedded PostgreSQL)
    #[allow(dead_code)] // Used for PostgreSQL migration
    pub async fn shutdown(&mut self) -> Result<(), BucketError> {
        if let Some(pg) = self.embedded_pg.take() {
            info!("Stopping embedded PostgreSQL...");
            pg.stop().await.map_err(|e| {
                BucketError::from(format!("Failed to stop PostgreSQL: {}", e).as_str())
            })?;
        }
        Ok(())
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
