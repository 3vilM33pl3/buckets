use crate::args::DoctorCommand;
use crate::commands::BucketCommand;
use crate::errors::BucketError;
use crate::postgres_db::DatabaseConfig;
use crate::utils::config::{GlobalConfig, RepositoryConfig};
use chrono::Utc;
use deadpool_postgres::{Config, Runtime};
use ntp::request;
use serde_json::json;
use std::net::ToSocketAddrs;
use std::time::{Duration, Instant};
use tokio_postgres::NoTls;

pub struct Doctor {
    args: DoctorCommand,
}

impl BucketCommand for Doctor {
    type Args = DoctorCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        if self.args.shared.json {
            self.execute_json()
        } else {
            self.execute_text()
        }
    }
}

impl Doctor {
    fn execute_text(&self) -> Result<(), BucketError> {
        println!("Buckets System Diagnostics");
        println!("==========================");
        println!();

        let mut all_passed = true;

        // Test database connection
        if !self.args.skip_database {
            match self.test_database_connection() {
                Ok(()) => {},
                Err(e) => {
                    all_passed = false;
                    if self.args.shared.verbose {
                        eprintln!("Database test failed: {}", e);
                    }
                }
            }
            println!();
        }

        // Test NTP server
        if !self.args.skip_ntp {
            match self.test_ntp_server() {
                Ok(()) => {},
                Err(e) => {
                    all_passed = false;
                    if self.args.shared.verbose {
                        eprintln!("NTP test failed: {}", e);
                    }
                }
            }
            println!();
        }

        // Summary
        println!("Summary");
        println!("-------");
        if all_passed {
            println!("✅ All systems operational");
        } else {
            println!("❌ Some issues detected");
            return Err(BucketError::from("System diagnostics failed"));
        }

        Ok(())
    }

    fn execute_json(&self) -> Result<(), BucketError> {
        let mut results = json!({
            "timestamp": Utc::now().to_rfc3339(),
            "tests": {}
        });

        let mut all_passed = true;

        // Test database connection
        if !self.args.skip_database {
            match self.test_database_connection_json() {
                Ok(result) => {
                    results["tests"]["database"] = result;
                }
                Err(e) => {
                    all_passed = false;
                    results["tests"]["database"] = json!({
                        "status": "failed",
                        "error": e.to_string()
                    });
                }
            }
        }

        // Test NTP server
        if !self.args.skip_ntp {
            match self.test_ntp_server_json() {
                Ok(result) => {
                    results["tests"]["ntp"] = result;
                }
                Err(e) => {
                    all_passed = false;
                    results["tests"]["ntp"] = json!({
                        "status": "failed",
                        "error": e.to_string()
                    });
                }
            }
        }

        results["summary"] = json!({
            "status": if all_passed { "passed" } else { "failed" },
            "all_passed": all_passed
        });

        println!("{}", serde_json::to_string_pretty(&results).map_err(|e| {
            BucketError::from(format!("Failed to serialize JSON: {}", e).as_str())
        })?);

        if !all_passed {
            return Err(BucketError::from("System diagnostics failed"));
        }

        Ok(())
    }

    fn test_database_connection(&self) -> Result<(), BucketError> {
        println!("Database Connection Test");
        println!("------------------------");

        let connection_string = if self.args.use_repo {
            // Try to get repository config
            let current_dir = std::env::current_dir().map_err(|e| {
                BucketError::from(format!("Failed to get current directory: {}", e).as_str())
            })?;
            
            match RepositoryConfig::from_file(current_dir) {
                Ok(config) => {
                    match config.postgresql_connection {
                        Some(conn) => {
                            println!("Using repository configuration");
                            conn
                        }
                        None => {
                            println!("❌ No PostgreSQL connection configured in repository");
                            return Err(BucketError::from("No repository PostgreSQL configuration found"));
                        }
                    }
                }
                Err(_) => {
                    println!("❌ Not in a Buckets repository or no configuration found");
                    return Err(BucketError::from("Cannot find repository configuration"));
                }
            }
        } else {
            // Use global config
            match GlobalConfig::load() {
                Ok(config) => {
                    match config.postgresql_connection {
                        Some(conn) => {
                            println!("Using global configuration");
                            conn
                        }
                        None => {
                            println!("❌ No PostgreSQL connection configured globally");
                            println!("   Run 'buckets setup' to configure a database connection");
                            return Err(BucketError::from("No global PostgreSQL configuration found"));
                        }
                    }
                }
                Err(_) => {
                    println!("❌ No global configuration found");
                    println!("   Run 'buckets setup' to create global configuration");
                    return Err(BucketError::from("No global configuration found"));
                }
            }
        };

        // Mask password in display
        let display_conn = self.mask_password(&connection_string);
        println!("Testing connection: {}", display_conn);

        let start = Instant::now();
        
        // Test connection using tokio runtime
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            BucketError::from(format!("Failed to create async runtime: {}", e).as_str())
        })?;

        rt.block_on(async {
            self.test_postgresql_connection(&connection_string).await
        })?;

        let duration = start.elapsed();
        println!("✅ Database connection successful");
        println!("   Connection time: {}ms", duration.as_millis());

        Ok(())
    }

    fn test_database_connection_json(&self) -> Result<serde_json::Value, BucketError> {
        let connection_string = if self.args.use_repo {
            // Try to get repository config
            let current_dir = std::env::current_dir().map_err(|e| {
                BucketError::from(format!("Failed to get current directory: {}", e).as_str())
            })?;
            
            match RepositoryConfig::from_file(current_dir) {
                Ok(config) => {
                    config.postgresql_connection.ok_or_else(|| {
                        BucketError::from("No repository PostgreSQL configuration found")
                    })?
                }
                Err(_) => {
                    return Err(BucketError::from("Cannot find repository configuration"));
                }
            }
        } else {
            // Use global config
            match GlobalConfig::load() {
                Ok(config) => {
                    config.postgresql_connection.ok_or_else(|| {
                        BucketError::from("No global PostgreSQL configuration found")
                    })?
                }
                Err(_) => {
                    return Err(BucketError::from("No global configuration found"));
                }
            }
        };

        let start = Instant::now();
        
        // Test connection using tokio runtime
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            BucketError::from(format!("Failed to create async runtime: {}", e).as_str())
        })?;

        rt.block_on(async {
            self.test_postgresql_connection(&connection_string).await
        })?;

        let duration = start.elapsed();

        Ok(json!({
            "status": "passed",
            "connection_string": self.mask_password(&connection_string),
            "config_source": if self.args.use_repo { "repository" } else { "global" },
            "connection_time_ms": duration.as_millis(),
            "test_timestamp": Utc::now().to_rfc3339()
        }))
    }

    async fn test_postgresql_connection(&self, connection_string: &str) -> Result<(), BucketError> {
        // Parse connection string into database config
        let db_config = DatabaseConfig::from_url(connection_string)?;
        
        // Create connection configuration
        let mut cfg = Config::new();
        
        match db_config {
            DatabaseConfig::External {
                host,
                port,
                database,
                username,
                password,
            } => {
                cfg.host = Some(host);
                cfg.port = Some(port);
                cfg.user = Some(username);
                cfg.password = password;
                cfg.dbname = Some(database);
            }
            DatabaseConfig::Embedded { .. } => {
                return Err(BucketError::from("Cannot test embedded database connection from doctor command"));
            }
        }
        
        // Set connection timeout
        cfg.connect_timeout = Some(Duration::from_secs(10));
        
        // Create pool and test connection
        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls).map_err(|e| {
            BucketError::from(format!("Failed to create connection pool: {}", e).as_str())
        })?;
        
        // Test the connection
        let client = pool.get().await.map_err(|e| {
            BucketError::from(format!("Failed to connect to PostgreSQL database: {}", e).as_str())
        })?;
        
        // Try to execute a simple query
        client.execute("SELECT 1", &[]).await.map_err(|e| {
            BucketError::from(format!("Failed to execute test query: {}", e).as_str())
        })?;
        
        Ok(())
    }

    fn test_ntp_server(&self) -> Result<(), BucketError> {
        println!("NTP Server Test");
        println!("---------------");

        let ntp_server = self.get_ntp_server()?;
        println!("Testing NTP server: {}", ntp_server);

        let start = Instant::now();
        
        // Resolve the NTP server address
        let addr = format!("{}:123", ntp_server)
            .to_socket_addrs()
            .map_err(|e| BucketError::from(format!("Failed to resolve NTP server '{}': {}", ntp_server, e).as_str()))?
            .next()
            .ok_or_else(|| BucketError::from(format!("No address found for NTP server '{}'", ntp_server).as_str()))?;

        // Query NTP server
        let _result = request(addr).map_err(|e| {
            BucketError::from(format!("Failed to query NTP server: {}", e).as_str())
        })?;

        let duration = start.elapsed();
        
        println!("✅ NTP server reachable");
        println!("   Response time: {}ms", duration.as_millis());
        
        // Calculate basic info (NTP packet doesn't have offset method in this crate)
        // We can calculate basic offset from transmit and receive times
        println!("   NTP query successful");

        Ok(())
    }

    fn test_ntp_server_json(&self) -> Result<serde_json::Value, BucketError> {
        let ntp_server = self.get_ntp_server()?;
        let start = Instant::now();
        
        // Resolve the NTP server address
        let addr = format!("{}:123", ntp_server)
            .to_socket_addrs()
            .map_err(|e| BucketError::from(format!("Failed to resolve NTP server '{}': {}", ntp_server, e).as_str()))?
            .next()
            .ok_or_else(|| BucketError::from(format!("No address found for NTP server '{}'", ntp_server).as_str()))?;

        // Query NTP server
        let _result = request(addr).map_err(|e| {
            BucketError::from(format!("Failed to query NTP server: {}", e).as_str())
        })?;

        let duration = start.elapsed();

        Ok(json!({
            "status": "passed",
            "server": ntp_server,
            "config_source": if self.args.use_repo { "repository" } else { "global" },
            "response_time_ms": duration.as_millis(),
            "test_timestamp": Utc::now().to_rfc3339()
        }))
    }

    fn get_ntp_server(&self) -> Result<String, BucketError> {
        if self.args.use_repo {
            // Try to get repository config
            let current_dir = std::env::current_dir().map_err(|e| {
                BucketError::from(format!("Failed to get current directory: {}", e).as_str())
            })?;
            
            match RepositoryConfig::from_file(current_dir) {
                Ok(config) => Ok(config.ntp_server),
                Err(_) => {
                    Err(BucketError::from("Cannot find repository configuration"))
                }
            }
        } else {
            // Use global config
            match GlobalConfig::load() {
                Ok(config) => Ok(config.ntp_server),
                Err(_) => {
                    // Fallback to default if no global config
                    Ok("pool.ntp.org".to_string())
                }
            }
        }
    }

    fn mask_password(&self, connection_string: &str) -> String {
        if connection_string.contains("@") {
            let parts: Vec<&str> = connection_string.splitn(2, '@').collect();
            if parts.len() == 2 {
                let auth_part = parts[0];
                let host_part = parts[1];
                if let Some(colon_pos) = auth_part.rfind(':') {
                    format!("{}:***@{}", &auth_part[..colon_pos], host_part)
                } else {
                    connection_string.to_string()
                }
            } else {
                connection_string.to_string()
            }
        } else {
            connection_string.to_string()
        }
    }
}