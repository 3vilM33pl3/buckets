use crate::utils::checks;
use crate::utils::checks::find_directory_in_parents;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use toml::to_string;

#[derive(Serialize, Deserialize)]
pub(crate) struct RepositoryConfig {
    pub ntp_server: String,
    pub ip_check: String,
    pub url_check: String,
}

impl RepositoryConfig {
    pub(crate) fn from_file(path: PathBuf) -> Result<Self, std::io::Error> {
        let buckets_repo_path = find_directory_in_parents(&path, ".buckets")
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No .buckets directory found",
            ))?;

        let mut file = File::open(buckets_repo_path.join("config"))
            .map_err(
                |e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()),
            )?;
        let mut toml_string = String::new();
        file.read_to_string(&mut toml_string)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        Ok(toml::from_str(&toml_string).unwrap())
    }
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        RepositoryConfig {
            ntp_server: "pool.ntp.org".to_string(),
            ip_check: "8.8.8.8".to_string(),
            url_check: "api.ipify.org".to_string(),
        }
    }
}

pub fn get_db_conn() -> rusqlite::Result<Connection> {
    let current_path = env::current_dir().unwrap();
    let db_location = checks::db_location(current_path.as_path());
    Connection::open(db_location)
}

pub fn create_default_config(file_path: &Path) {
    println!("Creating config files in {:?}", file_path.as_os_str());

    let config = RepositoryConfig::default();
    let toml_string = to_string(&config).unwrap();
    let mut file = File::create(file_path.join("config")).unwrap();
    file.write_all(toml_string.as_bytes()).unwrap();
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_from_file() {
        let temp_dir = tempdir().unwrap();
        let buckets_dir = temp_dir.path().join(".buckets");
        fs::create_dir(&buckets_dir).unwrap();

        // Create and write to the file
        create_default_config(&buckets_dir.as_path());

        // Read the file
        let config = RepositoryConfig::from_file(temp_dir.path().to_path_buf()).unwrap();

        // Assertions
        assert_eq!(config.ip_check, "8.8.8.8");
        assert_eq!(config.ntp_server, "pool.ntp.org");
        assert_eq!(config.url_check, "api.ipify.org");
    }
}
