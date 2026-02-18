use crate::args::SetupCommand;
use crate::commands::BucketCommand;
use crate::errors::BucketError;
use crate::postgres_db::{create_connection_pool, DatabaseConfig, TlsConfig};
use crate::utils::config::GlobalConfig;
use crate::utils::runtime::RuntimeManager;
use dialoguer::{console::Term, theme::ColorfulTheme, Input, Select};
use std::path::Path;

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

        let mut config = GlobalConfig::load().unwrap_or_default();

        if self.args.test_connection {
            return self.test_database_connection(&config);
        }
        let items = vec![
            "Configure PostgreSQL",
            "Configure TLS Certificates",
            "Configure NTP Server",
            "Install PostgreSQL Extensions (pgvector)",
            "Test Connection",
            "Save & Exit",
            "Cancel",
        ];

        loop {
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
                    config = self.configure_tls(config)?;
                }
                Some(2) => {
                    config = self.configure_ntp_server(config)?;
                }
                Some(3) => {
                    if let Err(e) = self.install_extensions(&config) {
                        eprintln!("\n❌ Extension installation failed: {}", e);
                    }
                    // Pause to let user read output
                    if Input::<String>::new()
                        .with_prompt("Press Enter to continue")
                        .allow_empty(true)
                        .interact_text()
                        .is_ok()
                    {}
                }
                Some(4) => {
                    if let Err(e) = self.test_database_connection(&config) {
                        eprintln!("\n❌ Connection failed: {}", e);
                    }
                    // Pause to let user read output
                    if Input::<String>::new()
                        .with_prompt("Press Enter to continue")
                        .allow_empty(true)
                        .interact_text()
                        .is_ok()
                    {}
                }
                Some(5) => {
                    config.save()?;
                    println!("\n✅ Global configuration saved successfully!");
                    println!(
                        "Configuration file: {}",
                        GlobalConfig::config_path()?.display()
                    );
                    break;
                }
                Some(6) => {
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

    fn configure_tls(&self, mut config: GlobalConfig) -> Result<GlobalConfig, BucketError> {
        println!("\nTLS Certificate Configuration");
        println!("Leave blank to disable TLS.\n");

        let current_tls = config.tls.clone().unwrap_or_default();

        let ca_cert: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("CA certificate path")
            .default(current_tls.ca_cert.unwrap_or_default())
            .allow_empty(true)
            .interact_text()?;

        if ca_cert.trim().is_empty() {
            config.tls = None;
            println!("\nTLS disabled.");
            return Ok(config);
        }

        // Validate CA cert file exists
        if !Path::new(&ca_cert).exists() {
            eprintln!(
                "⚠️  Warning: CA certificate file '{}' does not exist",
                ca_cert
            );
        }

        let client_cert: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Client certificate path (blank for server-only TLS)")
            .default(current_tls.client_cert.unwrap_or_default())
            .allow_empty(true)
            .interact_text()?;

        let client_key: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Client private key path (blank for server-only TLS)")
            .default(current_tls.client_key.unwrap_or_default())
            .allow_empty(true)
            .interact_text()?;

        // Validate client cert/key files exist if provided
        if !client_cert.trim().is_empty() && !Path::new(&client_cert).exists() {
            eprintln!(
                "⚠️  Warning: Client certificate file '{}' does not exist",
                client_cert
            );
        }
        if !client_key.trim().is_empty() && !Path::new(&client_key).exists() {
            eprintln!(
                "⚠️  Warning: Client key file '{}' does not exist",
                client_key
            );
        }

        let tls = TlsConfig {
            ca_cert: Some(ca_cert),
            client_cert: if client_cert.trim().is_empty() {
                None
            } else {
                Some(client_cert)
            },
            client_key: if client_key.trim().is_empty() {
                None
            } else {
                Some(client_key)
            },
        };

        if tls.is_mtls() {
            println!("\nmTLS configured (server verification + client authentication).");
        } else {
            println!("\nServer-verified TLS configured (no client authentication).");
        }

        config.tls = Some(tls);
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

    /// Build a DatabaseConfig from a GlobalConfig (connection string + TLS)
    fn db_config_from_global(config: &GlobalConfig) -> Result<DatabaseConfig, BucketError> {
        let conn_str = config
            .postgresql_connection
            .as_ref()
            .ok_or_else(|| BucketError::from("No PostgreSQL connection string configured"))?;
        let mut db_config = DatabaseConfig::from_url(conn_str)?;
        db_config.tls = config.tls.clone();
        Ok(db_config)
    }

    fn install_extensions(&self, config: &GlobalConfig) -> Result<(), BucketError> {
        println!("\nInstalling PostgreSQL Extensions...");

        let db_config = Self::db_config_from_global(config)?;
        RuntimeManager::block_on(async { self.install_pgvector(&db_config).await })?;
        println!("✅ Extension installation completed!");

        Ok(())
    }

    async fn install_pgvector(&self, db_config: &DatabaseConfig) -> Result<(), BucketError> {
        let pool = create_connection_pool(db_config)?;

        let client = pool.get().await.map_err(|e| {
            BucketError::from(format!("Failed to connect to PostgreSQL database: {}", e).as_str())
        })?;

        println!("   Enabling 'vector' extension...");
        client
            .execute("CREATE EXTENSION IF NOT EXISTS vector", &[])
            .await
            .map_err(|e| {
                BucketError::from(format!("Failed to enable vector extension: {}", e).as_str())
            })?;
        println!("   ✅ 'vector' extension enabled");

        Ok(())
    }

    fn test_database_connection(&self, config: &GlobalConfig) -> Result<(), BucketError> {
        println!("\nTesting Database Connection...");

        let db_config = Self::db_config_from_global(config)?;
        if db_config.tls.as_ref().is_some_and(|t| t.is_enabled()) {
            let tls = db_config.tls.as_ref().expect("tls checked above");
            if tls.is_mtls() {
                println!("TLS mode: mTLS (mutual TLS)");
            } else {
                println!("TLS mode: server-verified TLS");
            }
        } else {
            println!("TLS mode: disabled");
        }

        RuntimeManager::block_on(async { self.test_postgresql_connection(&db_config).await })?;
        println!("✅ PostgreSQL connection successful!");

        Ok(())
    }

    async fn test_postgresql_connection(
        &self,
        db_config: &DatabaseConfig,
    ) -> Result<(), BucketError> {
        let pool = create_connection_pool(db_config)?;

        pool.get()
            .await
            .map_err(|e| {
                BucketError::from(
                    format!("Failed to connect to PostgreSQL database: {}", e).as_str(),
                )
            })?
            .execute("SELECT 1", &[])
            .await
            .map_err(|e| {
                BucketError::from(format!("Failed to execute test query: {}", e).as_str())
            })?;

        Ok(())
    }
}
