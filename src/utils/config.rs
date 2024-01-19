use crate::utils::checks::find_directory_in_parents;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use toml::to_string;

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    pub ntp_server: String,
    pub ip_check: String,
    pub url_check: String,
}

impl Config {
    fn default() -> Self {
        Config {
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
        let config: Config = toml::from_str(&config_str).unwrap();
        config
    }
}

pub fn create_default_config(file_path: &Path) {
    println!("Creating config files in {:?}", file_path.as_os_str());

    let config = Config::default();
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
        let config = Config::from_file(temp_dir.path().to_path_buf());

        // Assertions
        assert_eq!(config.ip_check, "8.8.8.8");
        assert_eq!(config.ntp_server, "pool.ntp.org");
        assert_eq!(config.url_check, "api.ipify.org");
    }
}
