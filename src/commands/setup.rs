use crate::args::SetupCommand;
use crate::commands::BucketCommand;
use crate::errors::BucketError;
use crate::postgres_db::DatabaseConfig;
use crate::utils::config::GlobalConfig;
use crate::utils::runtime::RuntimeManager;
use deadpool_postgres::{Config, Runtime};
use std::io::{self, Write};
use std::time::Duration;
use tokio_postgres::NoTls;

pub struct Setup {
    args: SetupCommand,
}

impl BucketCommand for Setup {
    type Args = SetupCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        if self.args.shared.json {
            if self.args.test_connection {
                return Err(BucketError::InvalidData(
                    "--json cannot be combined with --test-connection".to_string(),
                ));
            }

            let config = GlobalConfig::load().unwrap_or_default();
            let json = serde_json::to_string_pretty(&config)
                .map_err(|e| BucketError::InvalidData(e.to_string()))?;
            println!("{}", json);
            return Ok(());
        }

        println!("Buckets Global Configuration Setup");
        println!("=================================");
        println!();

        // Load existing config or create new one
        let mut config = GlobalConfig::load().unwrap_or_default();
        
        // Interactive configuration
        config = self.configure_postgresql(config)?;
        config = self.configure_ntp_server(config)?;

        // Save configuration
        config.save()?;
        
        println!();
        println!("Global configuration saved successfully!");
        println!("Configuration file: {}", GlobalConfig::config_path()?.display());
        
        // Test database connection if requested
        if self.args.test_connection {
            println!();
            self.test_database_connection(&config)?;
        }
        
        Ok(())
    }
}

impl Setup {
    fn configure_postgresql(&self, mut config: GlobalConfig) -> Result<GlobalConfig, BucketError> {
        println!("PostgreSQL Configuration");
        println!("------------------------");
        
        if let Some(ref current) = config.postgresql_connection {
            println!("Current PostgreSQL connection string: {}", current);
        } else {
            println!("No PostgreSQL connection string configured");
        }
        
        print!("Enter PostgreSQL connection string (or press Enter to keep current): ");
        io::stdout().flush().map_err(|e| BucketError::IoError(e))?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|e| BucketError::IoError(e))?;
        let input = input.trim();
        
        if !input.is_empty() {
            config.postgresql_connection = Some(input.to_string());
            println!("PostgreSQL connection string updated");
        } else if config.postgresql_connection.is_some() {
            println!("Keeping existing PostgreSQL connection string");
        }
        
        println!();
        Ok(config)
    }
    
    fn configure_ntp_server(&self, mut config: GlobalConfig) -> Result<GlobalConfig, BucketError> {
        println!("NTP Server Configuration");
        println!("------------------------");
        
        println!("Current NTP server: {}", config.ntp_server);
        print!("Enter NTP server (or press Enter to keep current): ");
        io::stdout().flush().map_err(|e| BucketError::IoError(e))?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|e| BucketError::IoError(e))?;
        let input = input.trim();
        
        if !input.is_empty() {
            config.ntp_server = input.to_string();
            println!("NTP server updated");
        } else {
            println!("Keeping existing NTP server");
        }
        
        println!();
        Ok(config)
    }
    
    fn test_database_connection(&self, config: &GlobalConfig) -> Result<(), BucketError> {
        println!("Testing Database Connection");
        println!("===========================");
        
        if let Some(ref conn_str) = config.postgresql_connection {
            println!("Testing PostgreSQL connection: {}", 
                     // Mask password in display
                     if conn_str.contains("@") {
                         let parts: Vec<&str> = conn_str.splitn(2, '@').collect();
                         if parts.len() == 2 {
                             let auth_part = parts[0];
                             let host_part = parts[1];
                             if let Some(colon_pos) = auth_part.rfind(':') {
                                 format!("{}:***@{}", &auth_part[..colon_pos], host_part)
                             } else {
                                 conn_str.clone()
                             }
                         } else {
                             conn_str.clone()
                         }
                     } else {
                         conn_str.clone()
                     });
            
            RuntimeManager::block_on(async {
                self.test_postgresql_connection(conn_str).await
            })?;
            
            println!("✅ PostgreSQL connection successful!");
        } else {
            println!("No PostgreSQL connection string configured to test.");
            println!("Configure a PostgreSQL connection first, then use --test-connection");
        }
        
        Ok(())
    }
    
    async fn test_postgresql_connection(&self, connection_string: &str) -> Result<(), BucketError> {
        // Parse connection string into database config
        let db_config = DatabaseConfig::from_url(connection_string)?;
        
        // Create connection configuration
        let mut cfg = Config::new();
        
        cfg.host = Some(db_config.host);
        cfg.port = Some(db_config.port);
        cfg.user = Some(db_config.username);
        cfg.password = db_config.password;
        cfg.dbname = Some(db_config.database);
        
        // Set connection timeout
        cfg.connect_timeout = Some(Duration::from_secs(10));
        
        // Create pool and test connection
        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls).map_err(|e| {
            BucketError::from(format!("Failed to create connection pool: {}", e).as_str())
        })?;
        
        // Test the connection
        let _conn = pool.get().await.map_err(|e| {
            BucketError::from(format!("Failed to connect to PostgreSQL database: {}", e).as_str())
        })?;
        
        // Try to execute a simple query
        let client = pool.get().await.map_err(|e| {
            BucketError::from(format!("Failed to get database connection: {}", e).as_str())
        })?;
        
        client.execute("SELECT 1", &[]).await.map_err(|e| {
            BucketError::from(format!("Failed to execute test query: {}", e).as_str())
        })?;
        
        Ok(())
    }
}