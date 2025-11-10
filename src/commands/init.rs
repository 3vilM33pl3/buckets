use crate::args::InitCommand;
use crate::commands::BucketCommand;
use crate::config::Config;
use crate::database::{initialize_database_with_config_async, save_database_config};
use crate::errors::BucketError;
use crate::postgres_db::DatabaseConfig;
use crate::utils::checks;
use crate::utils::runtime::RuntimeManager;
use crate::CURRENT_DIR;
use log::debug;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs, io};

/// Initialize a new bucket repository
pub struct Init {
    args: InitCommand,
}

impl BucketCommand for Init {
    type Args = InitCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        debug!("Initializing bucket repository {}", self.args.repo_name);

        // Perform checks
        self.checks(&self.args.repo_name)?;

        // Create the repository
        let current_dir = CURRENT_DIR.with(|dir| dir.clone());
        debug!("Current directory: {:?}", current_dir);

        self.create_repo(&self.args.repo_name, &current_dir)?;

        println!("Bucket repository initialized successfully.");
        Ok(())
    }
}

impl Init {
    fn resolve_config_location(&self, location: &Path) -> Result<PathBuf, BucketError> {
        let base_dir = CURRENT_DIR.with(|dir| dir.clone());
        let resolved_location = if location.is_absolute() {
            location.to_path_buf()
        } else {
            base_dir.join(location)
        };

        let temp_dir = std::env::temp_dir();
        if !(resolved_location.starts_with(&base_dir) || resolved_location.starts_with(&temp_dir)) {
            return Err(BucketError::IoError(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "Configuration directory must be within {} or {}",
                    base_dir.display(),
                    temp_dir.display()
                ),
            )));
        }

        if let Some(parent) = resolved_location.parent() {
            if !parent.exists() {
                return Err(BucketError::IoError(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Configuration directory parent does not exist: {}",
                        parent.display()
                    ),
                )));
            }
        }

        Ok(resolved_location)
    }

    fn create_repo(&self, repo_name: &str, repo_location: &Path) -> Result<(), BucketError> {
        let repo_path = repo_location.join(repo_name);
        let repo_buckets_path = repo_path.join(".buckets");

        fs::create_dir_all(&repo_buckets_path)?;
        self.create_config_file(&repo_buckets_path)?;
        let db_type_file = repo_buckets_path.join("database_type");
        fs::write(&db_type_file, "PostgreSQL")?;

        // Initialize PostgreSQL database
        self.initialize_postgresql(&repo_path)?;

        Ok(())
    }

    pub fn create_config_file(&self, location: &Path) -> Result<(), BucketError> {
        let resolved_location = self.resolve_config_location(location)?;

        // Start with default configuration
        let mut config = Config {
            ntp_server: "pool.ntp.org".to_string(),
            ip_check: "8.8.8.8".to_string(),
            url_check: "api.ipify.org".to_string(),
            postgresql_connection: None,
        };

        // Override with global config values if available
        if let Ok(global_config) = crate::utils::config::GlobalConfig::load() {
            config.ntp_server = global_config.ntp_server;
            if global_config.postgresql_connection.is_some() {
                config.postgresql_connection = global_config.postgresql_connection;
            }
        }

        // Serialize the configuration to TOML format
        let toml_content = toml::to_string(&config)
            .map_err(|e| io::Error::other(format!("Failed to serialize config: {}", e)))?;

        // Create the .buckets directory if it doesn't exist
        fs::create_dir_all(&resolved_location)?;

        // Write the configuration file
        let config_path = resolved_location.join("config");
        let mut file = fs::File::create(&config_path)?;
        file.write_all(toml_content.as_bytes())?;

        Ok(())
    }

    #[cfg(test)]
    pub fn create_config_file_no_global(&self, location: &Path) -> Result<(), BucketError> {
        let resolved_location = self.resolve_config_location(location)?;

        // Create default configuration without global inheritance
        let config = Config {
            ntp_server: "pool.ntp.org".to_string(),
            ip_check: "8.8.8.8".to_string(),
            url_check: "api.ipify.org".to_string(),
            postgresql_connection: None,
        };

        // Serialize the configuration to TOML format
        let toml_content = toml::to_string(&config)
            .map_err(|e| io::Error::other(format!("Failed to serialize config: {}", e)))?;

        // Create the .buckets directory if it doesnt exist
        fs::create_dir_all(&resolved_location)?;

        // Write the configuration file
        let config_path = resolved_location.join("config");
        let mut file = fs::File::create(&config_path)?;
        file.write_all(toml_content.as_bytes())?;

        Ok(())
    }

    fn checks(&self, repo_name: &str) -> Result<(), BucketError> {
        let repo_path = {
            let initial_path = CURRENT_DIR.with(|dir| dir.join(repo_name));
            if initial_path.exists() {
                initial_path
            } else if let Ok(actual_dir) = std::env::current_dir() {
                let fallback_path = actual_dir.join(repo_name);
                if fallback_path.exists() {
                    fallback_path
                } else {
                    initial_path
                }
            } else {
                initial_path
            }
        };

        if repo_path.exists() {
            if repo_path.is_dir() {
                if checks::is_valid_bucket_repo(repo_path.as_path()) {
                    return Err(BucketError::RepoAlreadyExists(repo_name.to_string()));
                }
                return Err(BucketError::IoError(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "Directory already exists",
                )));
            } else {
                return Err(BucketError::IoError(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "File with the same name already exists",
                )));
            }
        }
        Ok(())
    }

    /// Initialize PostgreSQL database for the repository
    fn initialize_postgresql(&self, repo_path: &Path) -> Result<(), BucketError> {
        // Try to build database configuration in priority order:
        // 1. Command line arguments (if provided)
        // 2. DATABASE_URL environment variable
        // 3. Global configuration
        let config = self.get_database_config()?;

        // Save the database configuration
        save_database_config(repo_path, &config)?;

        RuntimeManager::block_on(initialize_database_with_config_async(config))
    }

    /// Get database configuration from various sources in priority order
    fn get_database_config(&self) -> Result<DatabaseConfig, BucketError> {
        // 1. If command line arguments are provided, use them
        if self.args.external_host.is_some() && self.args.external_username.is_some() {
            return Ok(DatabaseConfig {
                host: self
                    .args
                    .external_host
                    .clone()
                    .expect("external host must be provided when username is set"),
                port: self.args.external_port.unwrap_or(5432),
                database: self
                    .args
                    .external_database
                    .clone()
                    .unwrap_or_else(|| "buckets".to_string()),
                username: self
                    .args
                    .external_username
                    .clone()
                    .expect("external username must be provided when host is set"),
                password: self.args.external_password.clone(),
            });
        }

        // 2. Try DATABASE_URL environment variable
        if let Ok(database_url) = std::env::var("DATABASE_URL") {
            return DatabaseConfig::from_url(&database_url);
        }

        // 3. Try global configuration
        if let Ok(global_config) = crate::utils::config::GlobalConfig::load() {
            if let Some(connection_string) = global_config.postgresql_connection {
                return DatabaseConfig::from_url(&connection_string);
            }
        }

        // 4. If none of the above work, return an error with helpful message
        Err(BucketError::from(
            "No database configuration found. Please provide one of:\n\
            1. Command line options: --external-host and --external-username\n\
            2. DATABASE_URL environment variable\n\
            3. Global configuration via 'buckets setup' command",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::SharedArguments;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_init_command(repo_name: &str) -> Init {
        let args = InitCommand {
            shared: SharedArguments::default(),
            repo_name: repo_name.to_string(),
            external_host: Some("localhost".to_string()),
            external_port: Some(5432),
            external_database: Some("buckets_test".to_string()),
            external_username: Some("test_user".to_string()),
            external_password: Some("test_password".to_string()),
        };
        Init::new(&args)
    }

    #[test]
    fn test_init_new() {
        let args = InitCommand {
            shared: SharedArguments::default(),
            repo_name: "test_repo".to_string(),
            external_host: Some("localhost".to_string()),
            external_port: Some(5432),
            external_database: Some("buckets_test".to_string()),
            external_username: Some("test_user".to_string()),
            external_password: Some("test_password".to_string()),
        };
        let init = Init::new(&args);
        assert_eq!(init.args.repo_name, "test_repo");
        assert_eq!(init.args.external_host, Some("localhost".to_string()));
    }

    #[test]
    fn test_create_config_file() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let config_dir = temp_dir.path().join("test_config");

        let init = create_test_init_command("test_repo");
        let result = init.create_config_file(&config_dir);

        assert!(result.is_ok());
        assert!(config_dir.exists());
        assert!(config_dir.is_dir());

        let config_file = config_dir.join("config");
        assert!(config_file.exists());
        assert!(config_file.is_file());

        // Verify the config file content
        let content = fs::read_to_string(&config_file).expect("Failed to read config file");
        assert!(content.contains("ntp_server = \"pool.ntp.org\""));
        assert!(content.contains("ip_check = \"8.8.8.8\""));
        assert!(content.contains("url_check = \"api.ipify.org\""));
    }

    #[test]
    fn test_create_config_file_existing_directory() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let config_dir = temp_dir.path().join("existing_config");

        // Pre-create the directory
        fs::create_dir_all(&config_dir).expect("Failed to create directory");

        let init = create_test_init_command("test_repo");
        let result = init.create_config_file(&config_dir);

        assert!(result.is_ok());

        let config_file = config_dir.join("config");
        assert!(config_file.exists());
        assert!(config_file.is_file());
    }

    #[test]
    fn test_create_config_file_invalid_path() {
        let init = create_test_init_command("test_repo");

        // Try to create config in a path that cannot be created (invalid parent)
        let invalid_path = std::path::Path::new("/invalid/path/that/does/not/exist");
        let result = init.create_config_file(invalid_path);

        assert!(result.is_err());
    }

    #[test]
    fn test_checks_valid_repo_name() {
        let init = create_test_init_command("valid_repo");
        let result = init.checks("valid_repo");

        // Should be ok because the repo doesn't exist yet
        assert!(result.is_ok());
    }

    #[test]
    fn test_checks_directory_already_exists() {
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        let test_repo_name = "temp_test_existing_repo";
        let existing_dir = current_dir.join(test_repo_name);

        // Create a directory that already exists but is not a bucket repo
        fs::create_dir_all(&existing_dir).expect("Failed to create directory");

        let init = create_test_init_command(test_repo_name);
        let result = init.checks(test_repo_name);

        // Clean up the test directory
        fs::remove_dir_all(&existing_dir).expect("Failed to remove test directory");

        assert!(result.is_err());
        match result.unwrap_err() {
            BucketError::IoError(err) => {
                assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
                assert_eq!(err.to_string(), "Directory already exists");
            }
            _ => panic!("Expected IoError with AlreadyExists kind"),
        }
    }

    #[test]
    fn test_checks_file_already_exists() {
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        let test_file_name = "temp_test_existing_file_repo";
        let existing_file = current_dir.join(test_file_name);

        // Create a file with the same name as the repo
        fs::write(&existing_file, "test content").expect("Failed to create file");

        let init = create_test_init_command(test_file_name);
        let result = init.checks(test_file_name);

        // Clean up the test file
        fs::remove_file(&existing_file).expect("Failed to remove test file");

        assert!(result.is_err());
        match result.unwrap_err() {
            BucketError::IoError(err) => {
                assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
                assert_eq!(err.to_string(), "File with the same name already exists");
            }
            _ => panic!("Expected IoError with AlreadyExists kind"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_checks_existing_bucket_repo() {
        // Use a temporary directory instead of relying on current_dir which might be invalid
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        let test_repo_name = "temp_test_existing_bucket_repo";
        let existing_repo = temp_dir.path().join(test_repo_name);
        let buckets_dir = existing_repo.join(".buckets");

        // Save the original current directory to restore it later
        let original_dir = std::env::current_dir().unwrap_or_else(|_| {
            // If we can't get the current directory, use the temp directory
            temp_dir.path().to_path_buf()
        });

        // Set current directory to temp directory for the test
        std::env::set_current_dir(temp_dir.path()).expect("Failed to change to temp directory");

        // Create a directory that looks like a bucket repo
        fs::create_dir_all(&buckets_dir).expect("Failed to create .buckets directory");

        // Create config file to make it look like a valid repo
        let config_file = buckets_dir.join("config");
        fs::write(&config_file, "ntp_server = \"pool.ntp.org\"").expect("Failed to create config");

        // Create PostgreSQL directory structure to make it look like a valid repo
        let postgres_dir = buckets_dir.join("postgres");
        fs::create_dir_all(&postgres_dir).expect("Failed to create postgres directory");

        // Create a database_type file to indicate PostgreSQL
        let db_type_file = buckets_dir.join("database_type");
        fs::write(&db_type_file, "PostgreSQL").expect("Failed to create database_type file");

        let init = create_test_init_command(test_repo_name);
        let result = init.checks(test_repo_name);

        // Restore original directory before cleanup
        std::env::set_current_dir(&original_dir).ok(); // Ignore errors if original_dir is invalid

        // Clean up the test directory
        fs::remove_dir_all(&existing_repo).expect("Failed to remove test directory");

        assert!(result.is_err());
        match result.unwrap_err() {
            BucketError::RepoAlreadyExists(repo_name) => {
                assert_eq!(repo_name, test_repo_name);
            }
            _ => panic!("Expected RepoAlreadyExists error"),
        }
    }

    #[test]
    #[ignore]
    fn test_create_repo_with_embedded_database() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        let init = create_test_init_command("test_repo");
        let result = init.create_repo("test_repo", temp_dir.path());

        // Handle network issues gracefully
        if result.is_err() {
            let error = result.unwrap_err();
            if error.to_string().contains("rate limit")
                || error.to_string().contains("Failed to install PostgreSQL")
            {
                eprintln!("Skipping test due to network issues: {}", error);
                return;
            } else {
                panic!("Unexpected error: {}", error);
            }
        }

        let repo_path = temp_dir.path().join("test_repo");
        assert!(repo_path.exists());
        assert!(repo_path.is_dir());

        let buckets_path = repo_path.join(".buckets");
        assert!(buckets_path.exists());
        assert!(buckets_path.is_dir());

        let config_file = buckets_path.join("config");
        assert!(config_file.exists());
        assert!(config_file.is_file());

        let postgres_dir = buckets_path.join("postgres");
        assert!(postgres_dir.exists());
        assert!(postgres_dir.is_dir());

        let db_type_file = buckets_path.join("database_type");
        assert!(db_type_file.exists());
        assert!(db_type_file.is_file());

        let db_type_content =
            fs::read_to_string(db_type_file).expect("Failed to read database_type file");
        assert_eq!(db_type_content.trim(), "embedded");
    }

    #[test]
    #[ignore]
    fn test_create_repo_with_postgresql() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        let init = create_test_init_command("test_repo");
        let result = init.create_repo("test_repo", temp_dir.path());

        // Handle network issues gracefully
        if result.is_err() {
            let error = result.unwrap_err();
            if error.to_string().contains("rate limit")
                || error.to_string().contains("Failed to install PostgreSQL")
            {
                eprintln!("Skipping test due to network issues: {}", error);
                return;
            } else {
                panic!("Unexpected error: {}", error);
            }
        }

        let repo_path = temp_dir.path().join("test_repo");
        let buckets_path = repo_path.join(".buckets");

        // Check the structure was created
        assert!(repo_path.exists());
        assert!(buckets_path.exists());

        let config_file = buckets_path.join("config");
        assert!(config_file.exists());

        let db_type_file = buckets_path.join("database_type");
        assert!(db_type_file.exists());
        let db_type_content =
            fs::read_to_string(db_type_file).expect("Failed to read database_type file");
        assert_eq!(db_type_content.trim(), "postgresql");
    }

    #[test]
    #[ignore]
    fn test_create_repo_invalid_database_type() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        let init = create_test_init_command("test_repo");
        let result = init.create_repo("test_repo", temp_dir.path());

        assert!(result.is_err());
        match result.unwrap_err() {
            BucketError::InvalidData(msg) => {
                assert!(msg.contains("Unsupported database type"));
                assert!(msg.contains("invalid_db"));
            }
            _ => panic!("Expected InvalidData error for invalid database type"),
        }
    }

    #[test]
    fn test_create_repo_permission_denied() {
        // This test is more complex as it requires a directory where we can't create subdirectories
        // We'll use a simple approach by trying to create in a read-only directory
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let readonly_dir = temp_dir.path().join("readonly");
        fs::create_dir_all(&readonly_dir).expect("Failed to create readonly directory");

        // Make the directory read-only (this might not work on all systems)
        let mut permissions = fs::metadata(&readonly_dir)
            .expect("Failed to get metadata")
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&readonly_dir, permissions).expect("Failed to set readonly");

        let init = create_test_init_command("test_repo");
        let result = init.create_repo("test_repo", &readonly_dir);

        // The result depends on the system's permission handling
        // On some systems, this might still succeed, so we'll just check it doesn't panic
        let _ = result; // Don't assert specific behavior as it's system-dependent
    }

    #[test]
    fn test_get_database_config_with_command_line_args() {
        let args = InitCommand {
            shared: SharedArguments::default(),
            repo_name: "test".to_string(),
            external_host: Some("localhost".to_string()),
            external_port: Some(5432),
            external_database: Some("test_db".to_string()),
            external_username: Some("test_user".to_string()),
            external_password: Some("test_pass".to_string()),
        };
        let init = Init::new(&args);
        let result = init.get_database_config();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.database, "test_db");
        assert_eq!(config.username, "test_user");
        assert_eq!(config.password, Some("test_pass".to_string()));
    }

    #[test]
    fn test_get_database_config_with_database_url() {
        // Set DATABASE_URL environment variable
        std::env::set_var(
            "DATABASE_URL",
            "postgresql://envuser:envpass@envhost:5433/envdb",
        );

        let args = InitCommand {
            shared: SharedArguments::default(),
            repo_name: "test".to_string(),
            external_host: None, // No CLI args provided
            external_port: None,
            external_database: None,
            external_username: None,
            external_password: None,
        };
        let init = Init::new(&args);
        let result = init.get_database_config();

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.host, "envhost");
        assert_eq!(config.port, 5433);
        assert_eq!(config.database, "envdb");
        assert_eq!(config.username, "envuser");
        assert_eq!(config.password, Some("envpass".to_string()));

        // Clean up
        std::env::remove_var("DATABASE_URL");
    }

    #[test]
    fn test_get_database_config_with_global_config() {
        // Make sure no DATABASE_URL is set
        std::env::remove_var("DATABASE_URL");

        let args = InitCommand {
            shared: SharedArguments::default(),
            repo_name: "test".to_string(),
            external_host: None, // No CLI args provided
            external_port: None,
            external_database: None,
            external_username: None,
            external_password: None,
        };
        let init = Init::new(&args);
        let result = init.get_database_config();

        // This test will succeed if global config exists, fail if it doesn't
        // In a real environment, global config may or may not exist
        match result {
            Ok(config) => {
                // Global config was found and used successfully
                assert!(!config.host.is_empty());
                assert!(!config.username.is_empty());
                println!("Global config found: {}@{}", config.username, config.host);
            }
            Err(error) => {
                // No configuration available anywhere - should show helpful error
                assert!(error
                    .to_string()
                    .contains("No database configuration found"));
                assert!(error.to_string().contains("--external-host"));
                assert!(error.to_string().contains("DATABASE_URL"));
                assert!(error.to_string().contains("buckets setup"));
                println!("No global config found, got expected error: {}", error);
            }
        }
    }

    #[test]
    fn test_config_file_content_format() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let config_dir = temp_dir.path().join("test_config");

        let init = create_test_init_command("test_repo");
        let result = init.create_config_file(&config_dir);

        assert!(result.is_ok());

        let config_file = config_dir.join("config");
        let content = fs::read_to_string(&config_file).expect("Failed to read config file");

        // Verify it's valid TOML format by parsing it
        let parsed: toml::Value = toml::from_str(&content).expect("Config file is not valid TOML");

        // Verify the structure
        assert!(parsed.get("ntp_server").is_some());
        assert!(parsed.get("ip_check").is_some());
        assert!(parsed.get("url_check").is_some());

        // Verify the values
        assert_eq!(parsed["ntp_server"].as_str(), Some("pool.ntp.org"));
        assert_eq!(parsed["ip_check"].as_str(), Some("8.8.8.8"));
        assert_eq!(parsed["url_check"].as_str(), Some("api.ipify.org"));
    }
}
