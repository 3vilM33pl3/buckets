use crate::errors::BucketError;
use crate::utils::checks::find_directory_in_parents;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use toml::Value;

fn load_toml_value(path: &PathBuf) -> Result<Value, std::io::Error> {
    let mut file = File::open(path)?;
    let mut toml_string = String::new();
    file.read_to_string(&mut toml_string)?;

    toml::from_str(&toml_string)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
}

fn get_string_at_path(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_str().map(|value| value.to_string())
}

fn get_string_with_fallback(value: &Value, nested: &[&str], flat: &str) -> Option<String> {
    get_string_at_path(value, nested).or_else(|| get_string_at_path(value, &[flat]))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RepositoryConfig {
    pub ntp_server: String,
    pub ip_check: String,
    pub url_check: String,
    pub postgresql_connection: Option<String>,
}

impl RepositoryConfig {
    pub fn from_file(path: PathBuf) -> Result<Self, std::io::Error> {
        Self::from_file_with_global_config(path, true)
    }

    fn from_file_with_global_config(
        path: PathBuf,
        use_global: bool,
    ) -> Result<Self, std::io::Error> {
        let buckets_repo_path = find_directory_in_parents(&path, ".buckets").ok_or(
            std::io::Error::new(std::io::ErrorKind::NotFound, "No .buckets directory found"),
        )?;

        let config_path = buckets_repo_path.join("config");
        let value = load_toml_value(&config_path)?;
        let mut config = RepositoryConfig {
            ntp_server: get_string_with_fallback(&value, &["network", "ntp_server"], "ntp_server")
                .unwrap_or_else(|| "pool.ntp.org".to_string()),
            ip_check: get_string_with_fallback(&value, &["network", "ip_check"], "ip_check")
                .unwrap_or_else(|| "8.8.8.8".to_string()),
            url_check: get_string_with_fallback(&value, &["network", "url_check"], "url_check")
                .unwrap_or_else(|| "api.ipify.org".to_string()),
            postgresql_connection: get_string_with_fallback(
                &value,
                &["database", "postgresql_connection"],
                "postgresql_connection",
            ),
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
    use std::fs;
    use tempfile::tempdir;

    /// Helper to write a default config file (same as Init::create_config_file_no_global)
    fn write_default_config(buckets_dir: &std::path::Path) {
        let config = crate::config::Config {
            network: crate::config::NetworkConfig {
                ntp_server: "pool.ntp.org".to_string(),
                ip_check: "8.8.8.8".to_string(),
                url_check: "api.ipify.org".to_string(),
            },
            database: None,
        };
        let toml_content = toml::to_string(&config).expect("Failed to serialize config");
        fs::create_dir_all(buckets_dir).expect("Failed to create .buckets dir");
        let config_path = buckets_dir.join("config");
        fs::write(config_path, toml_content).expect("Failed to write config");
    }

    #[test]
    fn test_from_file() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let buckets_dir = temp_dir.path().join(".buckets");

        write_default_config(&buckets_dir);

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
        assert_eq!(
            config.postgresql_connection,
            Some("postgresql://test:test@localhost:5432/test".to_string())
        );
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

        write_default_config(&buckets_dir);

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
pub struct GlobalConfig {
    pub ntp_server: String,
    pub postgresql_connection: Option<String>,
}

impl GlobalConfig {
    pub fn config_path() -> Result<PathBuf, BucketError> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            BucketError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find home directory",
            ))
        })?;
        Ok(home_dir.join(".buckets_config.toml"))
    }

    pub fn load() -> Result<Self, BucketError> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let value = load_toml_value(&config_path)?;
        Ok(Self {
            ntp_server: get_string_with_fallback(&value, &["network", "ntp_server"], "ntp_server")
                .unwrap_or_else(|| "pool.ntp.org".to_string()),
            postgresql_connection: get_string_with_fallback(
                &value,
                &["database", "postgresql_connection"],
                "postgresql_connection",
            ),
        })
    }

    pub fn save(&self) -> Result<(), BucketError> {
        let config_path = Self::config_path()?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(&self.to_toml_value()).map_err(|e| {
            BucketError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize global config: {}", e),
            ))
        })?;

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

impl GlobalConfig {
    fn to_toml_value(&self) -> Value {
        let mut root = toml::map::Map::new();

        let mut network = toml::map::Map::new();
        network.insert(
            "ntp_server".to_string(),
            Value::String(self.ntp_server.clone()),
        );
        root.insert("network".to_string(), Value::Table(network));

        let mut database = toml::map::Map::new();
        if let Some(connection) = &self.postgresql_connection {
            database.insert(
                "postgresql_connection".to_string(),
                Value::String(connection.clone()),
            );
        }
        if !database.is_empty() {
            root.insert("database".to_string(), Value::Table(database));
        }

        Value::Table(root)
    }
}
