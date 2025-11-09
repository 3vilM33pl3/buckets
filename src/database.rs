use crate::errors::BucketError;
use crate::postgres_db::{init_database, DatabaseConfig};
use std::fs;
use std::path::Path;

// Only external PostgreSQL databases are supported

/// Get the database configuration for the repository
/// Returns an error if no configuration exists - never defaults
#[allow(dead_code)] // Used for PostgreSQL migration
pub fn get_database_config(repo_path: &Path) -> Result<DatabaseConfig, BucketError> {
    let config_file = repo_path.join(".buckets").join("db_config.toml");

    if config_file.exists() {
        let content = fs::read_to_string(&config_file)?;
        parse_database_config(&content)
    } else {
        Err(BucketError::from("No database configuration found. Use 'buckets init' to initialize a repository with database settings."))
    }
}

/// Get the database configuration (no fallback - external database required)
#[allow(dead_code)] // Used for PostgreSQL migration
pub fn get_database_config_no_fallback(repo_path: &Path) -> Result<DatabaseConfig, BucketError> {
    get_database_config(repo_path)
}

/// Parse database configuration from TOML string
#[allow(dead_code)] // Used for PostgreSQL migration
fn parse_database_config(content: &str) -> Result<DatabaseConfig, BucketError> {
    let config: toml::Value = content
        .parse()
        .map_err(|e| BucketError::from(format!("Invalid database config: {}", e).as_str()))?;

    let db_type = config
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("external");

    if db_type != "external" {
        return Err(BucketError::from(
            "Only external PostgreSQL databases are supported. Embedded databases have been removed.",
        ));
    }

    let host = config
        .get("host")
        .and_then(|v| v.as_str())
        .ok_or_else(|| BucketError::from("Missing 'host' in database config"))?
        .to_string();

    let port = config
        .get("port")
        .and_then(|v| v.as_integer())
        .map(|p| p as u16)
        .unwrap_or(5432);

    let database = config
        .get("database")
        .and_then(|v| v.as_str())
        .unwrap_or("buckets")
        .to_string();

    let username = config
        .get("username")
        .and_then(|v| v.as_str())
        .ok_or_else(|| BucketError::from("Missing 'username' in database config"))?
        .to_string();

    let password = config
        .get("password")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(DatabaseConfig {
        host,
        port,
        database,
        username,
        password,
    })
}

/// Save database configuration to file
pub fn save_database_config(repo_path: &Path, config: &DatabaseConfig) -> Result<(), BucketError> {
    let buckets_dir = repo_path.join(".buckets");
    std::fs::create_dir_all(&buckets_dir).map_err(|e| BucketError::from(e))?;
    let config_file = buckets_dir.join("db_config.toml");

    let mut content = String::from("type = \"external\"\n");
    content.push_str(&format!("host = \"{}\"\n", config.host));
    content.push_str(&format!("port = {}\n", config.port));
    content.push_str(&format!("database = \"{}\"\n", config.database));
    content.push_str(&format!("username = \"{}\"\n", config.username));
    if let Some(pwd) = &config.password {
        content.push_str(&format!("password = \"{}\"\n", pwd));
    }

    fs::write(config_file, content)?;
    Ok(())
}

/// Synchronous wrapper for initialize_database
#[allow(dead_code)] // Used for PostgreSQL migration
pub fn initialize_database_with_config(config: DatabaseConfig) -> Result<(), BucketError> {
    // Create a simple runtime for this operation
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        BucketError::from(format!("Failed to create async runtime: {}", e).as_str())
    })?;

    rt.block_on(initialize_database_with_config_async(config))
}

/// Initialize database with external configuration
pub async fn initialize_database_with_config_async(
    config: DatabaseConfig,
) -> Result<(), BucketError> {
    // Validate configuration
    if config.host.is_empty() {
        return Err(BucketError::from("Database host cannot be empty"));
    }
    if config.username.is_empty() {
        return Err(BucketError::from("Database username cannot be empty"));
    }

    // Initialize the database
    init_database(config).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_get_database_config_fails_without_config() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let buckets_dir = temp_dir.path().join(".buckets");
        std::fs::create_dir_all(&buckets_dir).expect("Failed to create .buckets dir");

        // Should fail because no config file exists
        let result = get_database_config(&buckets_dir);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No database configuration found"));
    }

    #[test]
    fn test_get_database_config_no_fallback_fails() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let buckets_dir = temp_dir.path().join(".buckets");
        std::fs::create_dir_all(&buckets_dir).expect("Failed to create .buckets dir");

        // Should fail because no config file exists and no fallback
        let result = get_database_config_no_fallback(&buckets_dir);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No database configuration found"));
    }

    #[test]
    fn test_parse_embedded_database_config_fails() {
        let config_content = r#"
type = "embedded"
port = 5432
"#;

        let result = parse_database_config(config_content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Only external PostgreSQL databases are supported"));
    }

    #[test]
    fn test_parse_external_database_config_missing_host() {
        let config_content = r#"
type = "external"
port = 5432
database = "buckets"
username = "user"
"#;

        let result = parse_database_config(config_content);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'host'"));
    }

    #[test]
    fn test_parse_external_database_config_missing_username() {
        let config_content = r#"
type = "external"
host = "localhost"
port = 5432
database = "buckets"
"#;

        let result = parse_database_config(config_content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing 'username'"));
    }
}
