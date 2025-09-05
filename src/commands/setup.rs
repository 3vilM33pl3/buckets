use crate::args::SetupCommand;
use crate::commands::BucketCommand;
use crate::errors::BucketError;
use crate::utils::config::GlobalConfig;
use std::io::{self, Write};

pub struct Setup {
    _args: SetupCommand,
}

impl BucketCommand for Setup {
    type Args = SetupCommand;

    fn new(args: &Self::Args) -> Self {
        Self { _args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
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
}