use crate::utils::checks;
use crate::utils::config::{get_db_conn, BucketConfig, RepositoryConfig};
use crate::utils::errors::BucketError;
use std::env;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

pub fn execute(bucket_name: &String) -> Result<(), BucketError> {
    #[allow(unused_variables)]
        let repo_config = RepositoryConfig::from_file(env::current_dir()?).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error reading repository config: {}", e),
        )
    })?;
    let db_conn = get_db_conn().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error opening database: {}", e),
        )
    })?;

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

    let relative_path = to_relative_path(current_path.as_path(), path.as_path()).unwrap();

    db_conn
        .execute(
            "INSERT INTO buckets (name, path) VALUES (?1, ?2)",
            &[&bucket_name, relative_path.to_str().unwrap()],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error inserting into database: {}", e),
            )
        })?;

    let mut stmt = db_conn
        .prepare("SELECT id FROM buckets WHERE name = ?1 AND path = ?2")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error preparing statement: {}", e),
            )
        })?;

    let bucket_id_str: String = stmt
        .query_row(&[&bucket_name, relative_path.to_str().unwrap()], |row| {
            row.get(0)
        })
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error querying statement: {}", e),
            )
        })?;

    // add info to bucket hidden directory
    let bucket_id = uuid::Uuid::parse_str(&bucket_id_str).unwrap();
    let config = BucketConfig::default(bucket_id, bucket_name, &relative_path);
    config.write_bucket_info();

    Ok(())
}

fn to_relative_path(repo_base: &Path, absolute_path: &Path) -> Option<PathBuf> {
    absolute_path
        .strip_prefix(repo_base)
        .ok()
        .map(PathBuf::from)
}
