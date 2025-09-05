use crate::utils::checks::find_directory_in_parents;
use crate::errors::BucketError;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RepositoryConfig {
    pub ntp_server: String,
    pub ip_check: String,
    pub url_check: String,
    pub postgresql_connection: Option<String>,
}

impl RepositoryConfig {
    pub(crate) fn from_file(path: PathBuf) -> Result<Self, std::io::Error> {
        Self::from_file_with_global_config(path, true)
    }

    fn from_file_with_global_config(path: PathBuf, use_global: bool) -> Result<Self, std::io::Error> {
        let buckets_repo_path = find_directory_in_parents(&path, ".buckets").ok_or(
            std::io::Error::new(std::io::ErrorKind::NotFound, "No .buckets directory found"),
        )?;

        let mut file = File::open(buckets_repo_path.join("config"))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()))?;
        let mut toml_string = String::new();
        file.read_to_string(&mut toml_string)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        let mut config: RepositoryConfig = toml::from_str(&toml_string)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        // Override with global config values if available and requested
        if use_global {
            if let Ok(global_config) = GlobalConfig::load() {
                config.ntp_server = global_config.ntp_server;
                if global_config.postgresql_connection.is_some() {
                    config.postgresql_connection = global_config.postgresql_connection;
                }
            }
        }

        Ok(config)
    }

    /// Load config from file without global config influence (for tests)
    #[cfg(test)]
    fn from_file_no_global(path: PathBuf) -> Result<Self, std::io::Error> {
        Self::from_file_with_global_config(path, false)
    }
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self::default_with_global_config(true)
    }
}

impl RepositoryConfig {
    /// Create default config with option to use global config or not
    fn default_with_global_config(use_global: bool) -> Self {
        let mut config = RepositoryConfig {
            ntp_server: "pool.ntp.org".to_string(),
            ip_check: "8.8.8.8".to_string(),
            url_check: "api.ipify.org".to_string(),
            postgresql_connection: None,
        };

        // Override with global config values if available and requested
        if use_global {
            if let Ok(global_config) = GlobalConfig::load() {
                config.ntp_server = global_config.ntp_server;
                if global_config.postgresql_connection.is_some() {
                    config.postgresql_connection = global_config.postgresql_connection;
                }
            }
        }

        config
    }

    /// Create default config without global config influence (for tests)
    #[cfg(test)]
    fn default_no_global() -> Self {
        Self::default_with_global_config(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::BucketCommand;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_from_file() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let buckets_dir = temp_dir.path().join(".buckets");
        fs::create_dir(&buckets_dir).expect("Failed to create .buckets directory");

        // Create and write to the file
        let init_cmd = crate::commands::init::Init::new(&crate::args::InitCommand {
            shared: crate::args::SharedArguments::default(),
            repo_name: "test".to_string(),
            database: "embedded".to_string(),
        });
        init_cmd
            .create_config_file(&buckets_dir.as_path())
            .expect("Failed to create config file");

        // Read the file
        let config = RepositoryConfig::from_file_no_global(temp_dir.path().to_path_buf())
            .expect("Failed to read config file");

        // Assertions
        assert_eq!(config.ip_check, "8.8.8.8");
        assert_eq!(config.ntp_server, "pool.ntp.org");
        assert_eq!(config.url_check, "api.ipify.org");
        assert_eq!(config.postgresql_connection, None);
    }

    #[test]
    fn test_config_default_values() {
        let config = RepositoryConfig::default_no_global();
        assert_eq!(config.ntp_server, "pool.ntp.org");
        assert_eq!(config.ip_check, "8.8.8.8");
        assert_eq!(config.url_check, "api.ipify.org");
        assert_eq!(config.postgresql_connection, None);
    }

    #[test]
    fn test_config_serialization() {
        let config = RepositoryConfig::default_no_global();
        let serialized = toml::to_string(&config).expect("Failed to serialize config");

        assert!(serialized.contains("ntp_server"));
        assert!(serialized.contains("ip_check"));
        assert!(serialized.contains("url_check"));
        assert!(serialized.contains("pool.ntp.org"));
        assert!(serialized.contains("8.8.8.8"));
        assert!(serialized.contains("api.ipify.org"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_content = r#"
ntp_server = "custom.ntp.server"
ip_check = "1.1.1.1"
url_check = "custom.check.url"
postgresql_connection = "postgresql://test:test@localhost:5432/test"
"#;
        let config: RepositoryConfig =
            toml::from_str(toml_content).expect("Failed to deserialize config");

        assert_eq!(config.ntp_server, "custom.ntp.server");
        assert_eq!(config.ip_check, "1.1.1.1");
        assert_eq!(config.url_check, "custom.check.url");
        assert_eq!(config.postgresql_connection, Some("postgresql://test:test@localhost:5432/test".to_string()));
    }

    #[test]
    fn test_from_file_no_buckets_directory() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let result = RepositoryConfig::from_file(temp_dir.path().to_path_buf());

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert!(error.to_string().contains(".buckets"));
    }

    #[test]
    fn test_from_file_no_config_file() -> std::io::Result<()> {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let buckets_dir = temp_dir.path().join(".buckets");
        fs::create_dir(&buckets_dir)?;

        let result = RepositoryConfig::from_file(temp_dir.path().to_path_buf());
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn test_from_file_corrupted_config() -> std::io::Result<()> {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let buckets_dir = temp_dir.path().join(".buckets");
        fs::create_dir(&buckets_dir)?;

        // Write invalid TOML content
        let config_path = buckets_dir.join("config");
        fs::write(&config_path, "invalid toml content { [ ] }")?;

        let result = RepositoryConfig::from_file(temp_dir.path().to_path_buf());
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        Ok(())
    }

    #[test]
    fn test_from_file_nested_directory() -> std::io::Result<()> {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let buckets_dir = temp_dir.path().join(".buckets");
        fs::create_dir(&buckets_dir)?;

        // Create the config file
        let init_cmd = crate::commands::init::Init::new(&crate::args::InitCommand {
            shared: crate::args::SharedArguments::default(),
            repo_name: "test".to_string(),
            database: "embedded".to_string(),
        });
        init_cmd
            .create_config_file(&buckets_dir.as_path())
            .expect("Failed to create config file");

        // Create nested directory and test from there
        let nested_dir = temp_dir.path().join("nested").join("directory");
        fs::create_dir_all(&nested_dir)?;

        let config = RepositoryConfig::from_file_no_global(nested_dir)?;
        assert_eq!(config.ip_check, "8.8.8.8");
        assert_eq!(config.ntp_server, "pool.ntp.org");
        assert_eq!(config.url_check, "api.ipify.org");
        assert_eq!(config.postgresql_connection, None);
        Ok(())
    }

    #[test]
    fn test_config_debug_format() {
        let config = RepositoryConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("RepositoryConfig"));
        assert!(debug_str.contains("ntp_server"));
        assert!(debug_str.contains("ip_check"));
        assert!(debug_str.contains("url_check"));
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct GlobalConfig {
    pub ntp_server: String,
    pub postgresql_connection: Option<String>,
}

impl GlobalConfig {
    pub(crate) fn config_path() -> Result<PathBuf, BucketError> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| BucketError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound, 
                "Could not find home directory"
            )))?;
        Ok(home_dir.join(".buckets_config.toml"))
    }

    pub(crate) fn load() -> Result<Self, BucketError> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let mut file = File::open(&config_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        toml::from_str(&content)
            .map_err(|e| BucketError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to parse global config: {}", e)
            )))
    }

    pub(crate) fn save(&self) -> Result<(), BucketError> {
        let config_path = Self::config_path()?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| BucketError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize global config: {}", e)
            )))?;

        let mut file = File::create(&config_path)?;
        file.write_all(content.as_bytes())?;
        file.flush()?;

        Ok(())
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            ntp_server: "pool.ntp.org".to_string(),
            postgresql_connection: None,
        }
    }
}
