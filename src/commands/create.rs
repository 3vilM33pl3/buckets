use crate::utils::checks;
use crate::utils::config::BucketConfig;
use crate::utils::errors::BucketError;
use std::env;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

pub fn execute(bucket_name: &String) -> Result<(), BucketError> {
    let current_path = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            return Err(BucketError::IoError(e));
        }
    };

    // check if in buckets repository
    if !checks::is_valid_bucket_repo(current_path.as_path()) {
        return Err(BucketError::NotInBucketRepo);
    }

    // check if bucket already exists
    let path = current_path.join(bucket_name).join(".b");
    if path.exists() && path.is_dir() {
        return Err(BucketError::BucketAlreadyExists);
    }

    // create bucket with given name
    create_dir_all(path.as_path()).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error creating bucket: {}", e),
        )
    })?;

    // add info to bucket hidden directory
    let config = BucketConfig::default(&bucket_name, &path);
    config.write_bucket_config();

    let db_location = checks::db_location(current_path.as_path());
    let conn = rusqlite::Connection::open(db_location).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error opening database: {}", e),
        )
    })?;

    let relative_path = to_relative_path(current_path.as_path(), path.as_path()).unwrap();

    conn.execute(
        "INSERT INTO buckets (name, path) VALUES (?1, ?2)",
        &[&bucket_name, relative_path.to_str().unwrap()],
    )
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error inserting into database: {}", e),
        )
    })?;

    Ok(())
}

fn to_relative_path(repo_base: &Path, absolute_path: &Path) -> Option<PathBuf> {
    absolute_path
        .strip_prefix(repo_base)
        .ok()
        .map(PathBuf::from)
}

