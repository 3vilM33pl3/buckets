use crate::utils::checks;
use serde::Serialize;
use std::env;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use serde_derive::Deserialize;
use toml::to_string;
use crate::utils::errors::BucketError;

#[derive(Serialize, Deserialize)]
#[derive(PartialEq)]
#[derive(Debug)]
pub struct BucketConfig {
    pub name: String,
    pub path: PathBuf,
}

impl BucketConfig {
    fn default(name: &String, path: &PathBuf) -> Self {
        BucketConfig {
            name: name.to_string(),
            path: path.to_path_buf(),
        }
    }

    pub fn write_bucket_config(&self) {
        let toml_string = to_string(self).unwrap();
        let mut file = File::create(self.path.join("info")).unwrap();
        file.write_all(toml_string.as_bytes()).unwrap();
    }

    pub fn read_bucket_config(path: &PathBuf) -> Result<Self, std::io::Error> {
        let mut file = File::open(path.join("info"))?;
        let mut toml_string = String::new();
        file.read_to_string(&mut toml_string)?;

        let config = toml::from_str(&toml_string)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(config)
    }
}


pub fn execute(bucket_name: &String) -> Result<(), BucketError> {
    println!("Creating bucket");

    let current_path = match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            println!("Error getting current directory: {}", e);
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
    create_dir_all(path.as_path())?;

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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use super::*;

    #[test]
    fn test_default() {
        let name = String::from("test_bucket");
        let path = PathBuf::from("/some/path/.b");

        let config = BucketConfig::default(&name, &path);

        assert_eq!(config.name, name);
        assert_eq!(config.path, path);
    }

    #[test]
    fn test_write_and_read_bucket_config() -> std::io::Result<()> {
        let temp_dir = tempdir()?;
        let bucket_name = String::from("test_bucket");
        let bucket_path = temp_dir.path().to_path_buf();

        let config = BucketConfig::default(&bucket_name, &bucket_path);
        config.write_bucket_config();

        let read_config = BucketConfig::read_bucket_config(&bucket_path)?;

        assert_eq!(config, read_config);
        Ok(())
    }
}