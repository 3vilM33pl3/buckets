use crate::errors::BucketError;
use crate::postgres_db::{DatabaseConfig, init_database};
use std::path::Path;
use std::fs;

#[derive(Debug, Clone, Copy)]
pub enum DatabaseType {
    Embedded,    // Embedded PostgreSQL
    External,    // External PostgreSQL server
}

impl DatabaseType {
    pub fn from_str(s: &str) -> Result<Self, BucketError> {
        match s.to_lowercase().as_str() {
            "embedded" | "postgresql_embedded" => Ok(DatabaseType::Embedded),
            "external" | "postgresql" | "postgres" => Ok(DatabaseType::External),
            _ => Err(BucketError::InvalidData(format!(
                "Unsupported database type: {}. Use 'embedded' or 'external'",
                s
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseType::Embedded => "embedded",
            DatabaseType::External => "external",
        }
    }
}

/// Get the database configuration for the repository
pub fn get_database_config(repo_path: &Path) -> Result<DatabaseConfig, BucketError> {
    let config_file = repo_path.join(".buckets").join("db_config.toml");
    
    if config_file.exists() {
        let content = fs::read_to_string(&config_file)?;
        parse_database_config(&content, repo_path)
    } else {
        // Default to embedded PostgreSQL
        Ok(DatabaseConfig::Embedded {
            data_dir: repo_path.join(".buckets").join("postgres"),
            port: None,
        })
    }
}

/// Parse database configuration from TOML string
fn parse_database_config(content: &str, repo_path: &Path) -> Result<DatabaseConfig, BucketError> {
    let config: toml::Value = content.parse()
        .map_err(|e| BucketError::from(format!("Invalid database config: {}", e).as_str()))?;
    
    let db_type = config
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("embedded");
    
    match db_type {
        "embedded" => {
            let port = config
                .get("port")
                .and_then(|v| v.as_integer())
                .map(|p| p as u16);
            
            Ok(DatabaseConfig::Embedded {
                data_dir: repo_path.join(".buckets").join("postgres"),
                port,
            })
        }
        "external" => {
            let host = config
                .get("host")
                .and_then(|v| v.as_str())
                .ok_or_else(|| BucketError::from("Missing 'host' in external database config"))?
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
                .ok_or_else(|| BucketError::from("Missing 'username' in external database config"))?
                .to_string();
            
            let password = config
                .get("password")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            Ok(DatabaseConfig::External {
                host,
                port,
                database,
                username,
                password,
            })
        }
        _ => Err(BucketError::from(format!("Invalid database type: {}", db_type).as_str()))
    }
}

/// Save database configuration to file
pub fn save_database_config(repo_path: &Path, config: &DatabaseConfig) -> Result<(), BucketError> {
    let config_file = repo_path.join(".buckets").join("db_config.toml");
    
    let toml_content = match config {
        DatabaseConfig::Embedded { port, .. } => {
            let mut content = String::from("type = \"embedded\"\n");
            if let Some(p) = port {
                content.push_str(&format!("port = {}\n", p));
            }
            content
        }
        DatabaseConfig::External { host, port, database, username, password } => {
            let mut content = String::from("type = \"external\"\n");
            content.push_str(&format!("host = \"{}\"\n", host));
            content.push_str(&format!("port = {}\n", port));
            content.push_str(&format!("database = \"{}\"\n", database));
            content.push_str(&format!("username = \"{}\"\n", username));
            if let Some(pwd) = password {
                content.push_str(&format!("password = \"{}\"\n", pwd));
            }
            content
        }
    };
    
    fs::write(config_file, toml_content)?;
    Ok(())
}

/// Synchronous wrapper for initialize_database
pub fn initialize_database(repo_path: &Path, db_type: DatabaseType) -> Result<(), BucketError> {
    // Create a simple runtime for this operation
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| BucketError::from(format!("Failed to create async runtime: {}", e).as_str()))?;
    
    rt.block_on(initialize_database_async(repo_path, db_type))
}

/// Async version of initialize_database
pub async fn initialize_database_async(repo_path: &Path, db_type: DatabaseType) -> Result<(), BucketError> {
    let config = match db_type {
        DatabaseType::Embedded => DatabaseConfig::Embedded {
            data_dir: repo_path.join("postgres"),
            port: None,
        },
        DatabaseType::External => {
            // For external, try to get from config file or environment
            get_database_config(repo_path.parent().unwrap_or(repo_path))?
        }
    };
    
    // Save the config to file for future use
    save_database_config(repo_path.parent().unwrap_or(repo_path), &config)?;
    
    // Initialize the database
    init_database(config).await?;
    
    // Create a database type marker file for compatibility
    let db_type_file = repo_path.join("database_type");
    fs::write(db_type_file, db_type.as_str())?;
    
    Ok(())
}