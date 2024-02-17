use crate::utils::checks::find_directory_in_parents;
use serde::{Deserialize, Serialize};
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
    fn default() -> Self {
        RepositoryConfig {
            ntp_server: "pool.ntp.org".to_string(),
            ip_check: "8.8.8.8".to_string(),
            url_check: "api.ipify.org".to_string(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn from_file(current_path: PathBuf) -> Self {
        let file_path = find_directory_in_parents(current_path.as_path(), ".buckets").unwrap();
        let file = File::open(file_path.join("config")).unwrap();
        let config_str = file.bytes().map(|x| x.unwrap() as char).collect::<String>();
        let config: RepositoryConfig = toml::from_str(&config_str).unwrap();
        config
    }
}

pub fn create_default_config(file_path: &Path) {
    println!("Creating config files in {:?}", file_path.as_os_str());

    let config = RepositoryConfig::default();
    let toml_string = to_string(&config).unwrap();
    let mut file = File::create(file_path.join("config")).unwrap();
    file.write_all(toml_string.as_bytes()).unwrap();
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BucketConfig {
    pub name: String,
    pub path: PathBuf,
}

impl BucketConfig {
    pub(crate) fn default(name: &String, path: &PathBuf) -> Self {
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
        let config = RepositoryConfig::from_file(temp_dir.path().to_path_buf());

        // Assertions
        assert_eq!(config.ip_check, "8.8.8.8");
        assert_eq!(config.ntp_server, "pool.ntp.org");
        assert_eq!(config.url_check, "api.ipify.org");
    }

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
