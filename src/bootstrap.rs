use std::fs;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::errors::BucketError;
use crate::postgres_db::{init_database, DatabaseConfig};
use crate::utils::config::GlobalConfig;
use crate::utils::utils::find_bucket_repo;
use crate::CURRENT_DIR;

static BOOTSTRAPPED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

#[derive(Debug)]
enum ConnectionSource {
    Repository(PathBuf),
    Global(PathBuf),
}

impl ConnectionSource {
    fn path(&self) -> &Path {
        match self {
            ConnectionSource::Repository(path) | ConnectionSource::Global(path) => path,
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ConnectionSource::Repository(_) => "repository configuration",
            ConnectionSource::Global(_) => "global configuration",
        }
    }
}

#[derive(Deserialize)]
struct RepositoryConfigSnapshot {
    #[serde(default)]
    postgresql_connection: Option<String>,
}

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

    let repo_source = buckets_dir.join("config");
    let (connection_string, source) = match read_repository_connection(&repo_source)? {
        Some(connection) => (connection, ConnectionSource::Repository(repo_source)),
        None => {
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
                (connection, ConnectionSource::Global(global_path))
            } else {
                return Err(BucketError::IoError(Error::new(
                    ErrorKind::NotFound,
                    format!(
                        "No PostgreSQL connection found in {} or {}. Run 'buckets setup' or update the repository configuration.",
                        repo_source.display(),
                        global_path.display()
                    ),
                )));
            }
        }
    };

    let config = DatabaseConfig::from_url(&connection_string).map_err(|err| {
        BucketError::IoError(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Invalid PostgreSQL connection string in {}: {}",
                source.path().display(),
                err
            ),
        ))
    })?;

    let runtime = tokio::runtime::Runtime::new().map_err(|err| {
        BucketError::IoError(Error::new(
            ErrorKind::Other,
            format!(
                "Failed to create async runtime for database bootstrap: {}",
                err
            ),
        ))
    })?;

    let init_result = runtime.block_on(async { init_database(config).await });

    match init_result {
        Ok(()) => Ok(()),
        Err(err) => Err(BucketError::IoError(Error::new(
            ErrorKind::Other,
            format!(
                "Failed to initialize PostgreSQL using the {} at {}: {}",
                source.description(),
                source.path().display(),
                err
            ),
        ))),
    }
}

fn read_repository_connection(config_path: &Path) -> Result<Option<String>, BucketError> {
    if !config_path.is_file() {
        return Ok(None);
    }

    let contents = fs::read_to_string(config_path)?;
    let snapshot: RepositoryConfigSnapshot = toml::from_str(&contents).map_err(|err| {
        BucketError::IoError(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Failed to parse repository configuration at {}: {}",
                config_path.display(),
                err
            ),
        ))
    })?;

    Ok(snapshot.postgresql_connection)
}
