use crate::errors::BucketError;
use crate::postgres_db::{init_database, DatabaseConfig, TlsConfig};
use std::fs;
use std::path::{Path, PathBuf};

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

    // Parse optional [tls] section
    let tls = {
        let ca_cert = config
            .get("tls")
            .and_then(|v| v.get("ca_cert"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let client_cert = config
            .get("tls")
            .and_then(|v| v.get("client_cert"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let client_key = config
            .get("tls")
            .and_then(|v| v.get("client_key"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if ca_cert.is_some() || client_cert.is_some() || client_key.is_some() {
            Some(TlsConfig {
                ca_cert,
                client_cert,
                client_key,
            })
        } else {
            None
        }
    };

    let mut config = DatabaseConfig {
        host,
        port,
        database,
        username,
        password,
        tls,
    };
    if config.password.is_some() {
        log::warn!(
            "db_config.toml contains a database password. Move it out of the repository: \
             run `buckets init` again or set BUCKETS_DB_PASSWORD, then delete the password line."
        );
    } else {
        config.password = std::env::var("BUCKETS_DB_PASSWORD")
            .ok()
            .filter(|p| !p.is_empty())
            .or_else(|| lookup_password(&config))
            .or_else(|| global_config_password(&config));
    }
    Ok(config)
}

/// File holding database passwords outside any repository, readable only by
/// the user: `BUCKETS_CREDENTIALS_FILE`, or `~/.buckets_credentials.toml`.
pub fn credentials_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("BUCKETS_CREDENTIALS_FILE") {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    dirs::home_dir().map(|home| home.join(".buckets_credentials.toml"))
}

fn credential_key(config: &DatabaseConfig) -> String {
    format!(
        "{}@{}:{}/{}",
        config.username, config.host, config.port, config.database
    )
}

fn load_credentials(path: &Path) -> toml::Table {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
        .unwrap_or_default()
}

/// Password from the global config's connection string, when it points at
/// the same server and user.
fn global_config_password(config: &DatabaseConfig) -> Option<String> {
    let global = crate::utils::config::GlobalConfig::load().ok()?;
    let url = global.postgresql_connection?;
    let parsed = DatabaseConfig::from_url(&url).ok()?;
    (parsed.host == config.host && parsed.port == config.port && parsed.username == config.username)
        .then_some(parsed.password)
        .flatten()
}

/// The stored password for this user, host, port and database, if any.
pub fn lookup_password(config: &DatabaseConfig) -> Option<String> {
    let path = credentials_path()?;
    load_credentials(&path)
        .get(&credential_key(config))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

/// Save the config's password in the credentials file (mode 0600 on Unix).
pub fn store_password(config: &DatabaseConfig) -> Result<(), BucketError> {
    let Some(password) = &config.password else {
        return Ok(());
    };
    let path = credentials_path()
        .ok_or_else(|| BucketError::from("Could not find a home directory for credentials"))?;
    let mut table = load_credentials(&path);
    table.insert(
        credential_key(config),
        toml::Value::String(password.clone()),
    );
    let text = toml::to_string(&table).map_err(|e| BucketError::from(e.to_string().as_str()))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::utils::object_store::write_atomically(&path, |w| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            w.get_ref()
                .set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        std::io::Write::write_all(w, text.as_bytes())
    })?;
    Ok(())
}

/// Save database configuration to file
pub fn save_database_config(repo_path: &Path, config: &DatabaseConfig) -> Result<(), BucketError> {
    let buckets_dir = repo_path.join(".buckets");
    std::fs::create_dir_all(&buckets_dir)?;
    let config_file = buckets_dir.join("db_config.toml");

    let mut content = String::from("type = \"external\"\n");
    content.push_str(&format!("host = \"{}\"\n", config.host));
    content.push_str(&format!("port = {}\n", config.port));
    content.push_str(&format!("database = \"{}\"\n", config.database));
    content.push_str(&format!("username = \"{}\"\n", config.username));
    // The password never goes into the repository: it is kept in the user's
    // credentials file (see `credentials_path`) or supplied through
    // BUCKETS_DB_PASSWORD / DATABASE_URL.
    if config.password.is_some() {
        store_password(config)?;
    }

    if let Some(tls) = &config.tls {
        if tls.is_enabled() {
            content.push_str("\n[tls]\n");
            if let Some(ca) = &tls.ca_cert {
                content.push_str(&format!("ca_cert = \"{}\"\n", ca));
            }
            if let Some(cert) = &tls.client_cert {
                content.push_str(&format!("client_cert = \"{}\"\n", cert));
            }
            if let Some(key) = &tls.client_key {
                content.push_str(&format!("client_key = \"{}\"\n", key));
            }
        }
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
