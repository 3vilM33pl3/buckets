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

        let schema_result = rt.block_on(async {
            self.test_postgresql_connection_and_schema(&connection_string).await
        })?;

        let duration = start.elapsed();
        println!("✅ Database connection successful");
        println!("   Connection time: {}ms", duration.as_millis());
        
        // Display schema validation results
        self.display_schema_results(&schema_result)?;

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

        let schema_result = rt.block_on(async {
            self.test_postgresql_connection_and_schema(&connection_string).await
        })?;

        let duration = start.elapsed();

        let mut result = json!({
            "status": "passed",
            "connection_string": self.mask_password(&connection_string),
            "config_source": if self.args.use_repo { "repository" } else { "global" },
            "connection_time_ms": duration.as_millis(),
            "test_timestamp": Utc::now().to_rfc3339(),
            "schema_validation": schema_result
        });

        // Update overall status if schema validation failed
        if schema_result["overall_status"] == "failed" {
            result["status"] = json!("failed");
        }

        Ok(result)
    }


    async fn test_postgresql_connection_and_schema(&self, connection_string: &str) -> Result<serde_json::Value, BucketError> {
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
        
        // Validate database schema
        self.validate_database_schema(&client).await
    }

    fn display_schema_results(&self, schema_result: &serde_json::Value) -> Result<(), BucketError> {
        println!();
        println!("Database Schema Validation");
        println!("--------------------------");
        
        let overall_status = schema_result["overall_status"].as_str().unwrap_or("unknown");
        let empty_map = serde_json::Map::new();
        let tables = schema_result["tables"].as_object().unwrap_or(&empty_map);
        
        for (table_name, table_info) in tables {
            let status = table_info["status"].as_str().unwrap_or("unknown");
            let exists = table_info["exists"].as_bool().unwrap_or(false);
            
            if exists {
                let status_icon = if status == "passed" { "✅" } else { "❌" };
                println!("{} Table '{}': {}", status_icon, table_name, status);
                
                if let Some(missing_columns) = table_info["missing_columns"].as_array() {
                    if !missing_columns.is_empty() {
                        println!("   Missing columns:");
                        for missing in missing_columns {
                            if let (Some(name), Some(col_type)) = (missing["name"].as_str(), missing["type"].as_str()) {
                                println!("   - {} ({})", name, col_type);
                            }
                        }
                    }
                }
                
                if let Some(constraints) = table_info["constraints"].as_array() {
                    if !constraints.is_empty() {
                        println!("   Foreign key constraints: {}", constraints.len());
                    }
                }
            } else {
                println!("❌ Table '{}': does not exist", table_name);
                if let Some(error) = table_info["error"].as_str() {
                    println!("   Error: {}", error);
                }
            }
        }
        
        println!();
        if overall_status == "passed" {
            println!("✅ Database schema validation passed");
        } else {
            println!("❌ Database schema validation failed");
            println!("   Consider running database migrations to create/update tables");
        }
        
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

    async fn validate_database_schema(&self, client: &tokio_postgres::Client) -> Result<serde_json::Value, BucketError> {
        let mut results = json!({
            "tables": {},
            "overall_status": "passed"
        });

        let required_tables = vec!["buckets", "commits", "files"];
        let mut all_valid = true;

        for table_name in required_tables {
            match self.validate_table_structure(client, table_name).await {
                Ok(table_result) => {
                    if table_result["status"] != "passed" {
                        all_valid = false;
                    }
                    results["tables"][table_name] = table_result;
                }
                Err(e) => {
                    all_valid = false;
                    results["tables"][table_name] = json!({
                        "status": "failed",
                        "error": e.to_string(),
                        "exists": false
                    });
                }
            }
        }

        if !all_valid {
            results["overall_status"] = json!("failed");
        }

        Ok(results)
    }

    async fn validate_table_structure(&self, client: &tokio_postgres::Client, table_name: &str) -> Result<serde_json::Value, BucketError> {
        // Check if table exists
        let exists_query = "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables 
            WHERE table_schema = 'public' AND table_name = $1
        )";
        
        let row = client.query_one(exists_query, &[&table_name]).await.map_err(|e| {
            BucketError::from(format!("Failed to check if table '{}' exists: {}", table_name, e).as_str())
        })?;
        
        let table_exists: bool = row.get(0);
        
        if !table_exists {
            return Ok(json!({
                "status": "failed",
                "exists": false,
                "error": format!("Table '{}' does not exist", table_name)
            }));
        }

        // Get table columns
        let columns_query = "
            SELECT column_name, data_type, is_nullable, column_default
            FROM information_schema.columns 
            WHERE table_schema = 'public' AND table_name = $1
            ORDER BY ordinal_position
        ";
        
        let rows = client.query(columns_query, &[&table_name]).await.map_err(|e| {
            BucketError::from(format!("Failed to get columns for table '{}': {}", table_name, e).as_str())
        })?;

        let mut columns = Vec::new();
        for row in rows {
            let column_name: String = row.get(0);
            let data_type: String = row.get(1);
            let is_nullable: String = row.get(2);
            let column_default: Option<String> = row.get(3);
            
            columns.push(json!({
                "name": column_name,
                "type": data_type,
                "nullable": is_nullable == "YES",
                "default": column_default
            }));
        }

        // Check for expected columns based on table
        let expected_columns = self.get_expected_columns(table_name);
        let mut missing_columns = Vec::new();
        
        for expected in &expected_columns {
            let expected_name = expected["name"].as_str().unwrap_or("");
            let expected_type = expected["type"].as_str().unwrap_or("").to_lowercase();
            
            if !columns.iter().any(|col| {
                let col_name = col["name"].as_str().unwrap_or("");
                let col_type = col["type"].as_str().unwrap_or("").to_lowercase();
                col_name == expected_name && col_type.contains(&expected_type)
            }) {
                missing_columns.push(expected.clone());
            }
        }

        // Check foreign key constraints for tables that should have them
        let mut constraints = Vec::new();
        if table_name == "commits" || table_name == "files" {
            let fk_query = "
                SELECT tc.constraint_name, kcu.column_name, ccu.table_name, ccu.column_name
                FROM information_schema.table_constraints AS tc 
                JOIN information_schema.key_column_usage AS kcu
                    ON tc.constraint_name = kcu.constraint_name
                    AND tc.table_schema = kcu.table_schema
                JOIN information_schema.constraint_column_usage AS ccu
                    ON ccu.constraint_name = tc.constraint_name
                    AND ccu.table_schema = tc.table_schema
                WHERE tc.constraint_type = 'FOREIGN KEY'
                    AND tc.table_name = $1
                    AND tc.table_schema = 'public'
            ";
            
            let fk_rows = client.query(fk_query, &[&table_name]).await.map_err(|e| {
                BucketError::from(format!("Failed to get foreign keys for table '{}': {}", table_name, e).as_str())
            })?;

            for row in fk_rows {
                let constraint_name: String = row.get(0);
                let column_name: String = row.get(1);
                let referenced_table: String = row.get(2);
                let referenced_column: String = row.get(3);
                
                constraints.push(json!({
                    "name": constraint_name,
                    "column": column_name,
                    "references": format!("{}.{}", referenced_table, referenced_column)
                }));
            }
        }

        let status = if missing_columns.is_empty() {
            "passed"
        } else {
            "failed"
        };

        Ok(json!({
            "status": status,
            "exists": true,
            "columns": columns,
            "missing_columns": missing_columns,
            "constraints": constraints
        }))
    }

    fn get_expected_columns(&self, table_name: &str) -> Vec<serde_json::Value> {
        match table_name {
            "buckets" => vec![
                json!({"name": "id", "type": "uuid"}),
                json!({"name": "name", "type": "text"}),
                json!({"name": "path", "type": "text"}),
                json!({"name": "created_at", "type": "timestamp"})
            ],
            "commits" => vec![
                json!({"name": "id", "type": "uuid"}),
                json!({"name": "bucket_id", "type": "uuid"}),
                json!({"name": "message", "type": "text"}),
                json!({"name": "created_at", "type": "timestamp"})
            ],
            "files" => vec![
                json!({"name": "id", "type": "uuid"}),
                json!({"name": "commit_id", "type": "uuid"}),
                json!({"name": "file_path", "type": "text"}),
                json!({"name": "hash", "type": "text"})
            ],
            _ => vec![]
        }
    }
}