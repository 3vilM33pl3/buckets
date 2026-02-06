use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use once_cell::sync::Lazy;

use crate::errors::BucketError;
use crate::postgres_db::{init_database, DatabaseConfig};
use crate::utils::config::GlobalConfig;
use crate::utils::utils::find_bucket_repo;
use crate::CURRENT_DIR;

static BOOTSTRAPPED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub fn bootstrap_database() -> Result<(), BucketError> {
    if BOOTSTRAPPED.load(Ordering::SeqCst) {
        return Ok(());
    }

    match perform_bootstrap() {
        Ok(()) => {
            BOOTSTRAPPED.store(true, Ordering::SeqCst);
            Ok(())
        }
        Err(err) => {
            // If the database is already initialized we can treat this as success.
            if matches!(err, BucketError::DatabaseAlreadyInitialized) {
                BOOTSTRAPPED.store(true, Ordering::SeqCst);
                Ok(())
            } else {
                Err(err)
            }
        }
    }
}

fn perform_bootstrap() -> Result<(), BucketError> {
    let current_dir = CURRENT_DIR.with(|dir| dir.clone());
    let buckets_dir = find_bucket_repo(&current_dir).ok_or(BucketError::NotInRepo)?;

    // 1. Try DATABASE_URL first
    if let Ok(url) = std::env::var("DATABASE_URL") {
        let config = DatabaseConfig::from_url(&url)?;
        return initialize_runtime_and_db(
            config,
            "environment variable",
            &PathBuf::from("DATABASE_URL"),
        );
    }

    // 2. Try repository configuration (db_config.toml)
    if let Ok(config) =
        crate::database::get_database_config(buckets_dir.parent().unwrap_or(&buckets_dir))
    {
        return initialize_runtime_and_db(
            config,
            "repository configuration",
            &buckets_dir.join("db_config.toml"),
        );
    }

    // 3. Fallback to global configuration
    let global_path = GlobalConfig::config_path()?;
    let global_config = GlobalConfig::load().map_err(|err| {
        BucketError::IoError(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Failed to read global configuration at {}: {}",
                global_path.display(),
                err
            ),
        ))
    })?;

    if let Some(connection) = global_config.postgresql_connection {
        let config = DatabaseConfig::from_url(&connection)?;
        return initialize_runtime_and_db(config, "global configuration", &global_path);
    }

    Err(BucketError::IoError(Error::new(
        ErrorKind::NotFound,
        "No PostgreSQL connection found. Please set DATABASE_URL, run 'buckets init' with database options, or configure globally with 'buckets setup'.".to_string(),
    )))
}

fn initialize_runtime_and_db(
    config: DatabaseConfig,
    source_desc: &str,
    source_path: &Path,
) -> Result<(), BucketError> {
    let runtime = tokio::runtime::Runtime::new().map_err(|err| {
        BucketError::IoError(Error::other(format!(
            "Failed to create async runtime for database bootstrap: {}",
            err
        )))
    })?;

    let init_result = runtime.block_on(async { init_database(config).await });

    match init_result {
        Ok(()) => Ok(()),
        Err(err) => Err(BucketError::IoError(Error::other(format!(
            "Failed to initialize PostgreSQL using the {} at {}: {}",
            source_desc,
            source_path.display(),
            err
        )))),
    }
}
