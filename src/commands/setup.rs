use crate::args::SetupCommand;
use crate::commands::BucketCommand;
use crate::errors::BucketError;
use crate::postgres_db::DatabaseConfig;
use crate::utils::config::GlobalConfig;
use crate::utils::runtime::RuntimeManager;
use deadpool_postgres::{Config, Runtime};
use dialoguer::{console::Term, theme::ColorfulTheme, Input, Select};
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

        // Check if we are in a non-interactive environment (e.g. CI)
        // If so, fall back to non-interactive mode or error out if crucial info is missing
        // For now, we assume interactive unless testing

        let mut config = GlobalConfig::load().unwrap_or_default();
        let items = vec![
            "Configure PostgreSQL",
            "Configure NTP Server",
            "Test Connection",
            "Save & Exit",
            "Cancel",
        ];

        loop {
            // Clear screen for a cleaner look? Maybe just show menu.
            // Term::stdout().clear_screen().ok();

            println!("\nBuckets Configuration Setup");
            println!("===========================");

            let selection = Select::with_theme(&ColorfulTheme::default())
                .items(&items)
                .default(0)
                .interact_on_opt(&Term::stderr())?;

            match selection {
                Some(0) => {
                    config = self.configure_postgresql(config)?;
                }
                Some(1) => {
                    config = self.configure_ntp_server(config)?;
                }
                Some(2) => {
                    if let Err(e) = self.test_database_connection(&config) {
                        eprintln!("\n❌ Connection failed: {}", e);
                    } else {
                        // User needs to see the success message
                        // test_database_connection prints success message
                    }
                    // Pause to let user read output
                    if let Ok(_) = Input::<String>::new()
                        .with_prompt("Press Enter to continue")
                        .allow_empty(true)
                        .interact_text()
                    {}
                }
                Some(3) => {
                    config.save()?;
                    println!("\n✅ Global configuration saved successfully!");
                    println!(
                        "Configuration file: {}",
                        GlobalConfig::config_path()?.display()
                    );
                    break;
                }
                Some(4) => {
                    println!("\n❌ Configuration cancelled. Changes were not saved.");
                    break;
                }
                _ => break, // Handle ctrl-c or other interrupts
            }
        }

        Ok(())
    }
}

impl Setup {
    fn configure_postgresql(&self, mut config: GlobalConfig) -> Result<GlobalConfig, BucketError> {
        let current_conn = config.postgresql_connection.clone().unwrap_or_default();

        println!("\nPostgreSQL Configuration");

        let connection_string: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("PostgreSQL connection string")
            .default(current_conn.clone())
            .interact_text()?;

        if !connection_string.trim().is_empty() {
            config.postgresql_connection = Some(connection_string);
        }

        Ok(config)
    }

    fn configure_ntp_server(&self, mut config: GlobalConfig) -> Result<GlobalConfig, BucketError> {
        println!("\nNTP Server Configuration");

        let ntp_server: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("NTP Server")
            .default(config.ntp_server.clone())
            .interact_text()?;

        config.ntp_server = ntp_server;
        Ok(config)
    }

    fn test_database_connection(&self, config: &GlobalConfig) -> Result<(), BucketError> {
        println!("\nTesting Database Connection...");

        if let Some(ref conn_str) = config.postgresql_connection {
            RuntimeManager::block_on(async { self.test_postgresql_connection(conn_str).await })?;
            println!("✅ PostgreSQL connection successful!");
        } else {
            println!("⚠️ No PostgreSQL connection string configured.");
        }

        Ok(())
    }

    async fn test_postgresql_connection(&self, connection_string: &str) -> Result<(), BucketError> {
        let db_config = DatabaseConfig::from_url(connection_string)?;

        let mut cfg = Config::new();
        cfg.host = Some(db_config.host);
        cfg.port = Some(db_config.port);
        cfg.user = Some(db_config.username);
        cfg.password = db_config.password;
        cfg.dbname = Some(db_config.database);
        cfg.connect_timeout = Some(Duration::from_secs(10));

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls).map_err(|e| {
            BucketError::from(format!("Failed to create connection pool: {}", e).as_str())
        })?;

        let _conn = pool.get().await.map_err(|e| {
            BucketError::from(format!("Failed to connect to PostgreSQL database: {}", e).as_str())
        })?;

        pool.get()
            .await
            .map_err(|e| {
                BucketError::from(format!("Failed to get database connection: {}", e).as_str())
            })?
            .execute("SELECT 1", &[])
            .await
            .map_err(|e| {
                BucketError::from(format!("Failed to execute test query: {}", e).as_str())
            })?;

        Ok(())
    }
}
