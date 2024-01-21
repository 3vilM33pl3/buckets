use crate::utils::checks;
use serde::Serialize;
use std::env;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use toml::to_string;
#[derive(Serialize)]
struct BucketConfig {
    pub name: String,
}

impl BucketConfig {
    fn default(name: &String) -> Self {
        BucketConfig {
            name: name.to_string(),
        }
    }
}
pub fn execute(bucket_name: &String) -> Result<(), std::io::Error> {
    println!("Creating bucket");

    let current_path = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            println!("Error getting current directory: {}", e);
            return Err(e);
        }
    };

    // check if in buckets repository
    if !checks::is_valid_bucket_repo(current_path.as_path()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Not in a bucket repository",
        ));
    }

    // check if bucket already exists
    let path = current_path.join(bucket_name).join(".b");
    if path.exists() && path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Bucket already exists",
        ));
    }

    // create bucket with given name
    create_dir_all(path.as_path())?;

    // add info to bucket hidden directory
    let config = BucketConfig::default(bucket_name);
    let toml_string = to_string(&config).unwrap();
    let mut file = File::create(path.join("info")).unwrap();
    file.write_all(toml_string.as_bytes()).unwrap();

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
    ).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Error inserting into database: {}", e),
        )
    })?;

    Ok(())
}

fn to_relative_path(repo_base: &Path, absolute_path: &Path) -> Option<PathBuf> {
    absolute_path.strip_prefix(repo_base).ok().map(PathBuf::from)
}