use crate::args::{ConfigCommand, ConfigListCommand, ConfigSetCommand, ConfigSubcommand};
use crate::commands::BucketCommand;
use crate::errors::BucketError;
use crate::utils::checks::find_directory_in_parents;
use crate::utils::config::GlobalConfig;
use crate::CURRENT_DIR;
use std::fs;
use std::path::PathBuf;
use toml::Value;

const EMPTY_CONFIG_MESSAGE: &str = "No configuration values found.";

pub struct Config {
    args: ConfigCommand,
}

impl BucketCommand for Config {
    type Args = ConfigCommand;

    fn new(args: &Self::Args) -> Self {
        Self { args: args.clone() }
    }

    fn execute(&self) -> Result<(), BucketError> {
        match &self.args.command {
            ConfigSubcommand::Get(command) => self.get_value(&command.key, command.global, command.local),
            ConfigSubcommand::Set(command) => self.set_value(command),
            ConfigSubcommand::Unset(command) => self.unset_value(&command.key, command.global, command.local),
            ConfigSubcommand::List(command) => self.list_values(command),
        }
    }
}

impl Config {
    fn get_value(&self, key: &str, global: bool, local: bool) -> Result<(), BucketError> {
        let value = if global {
            let config = load_config_value(config_path_global()?)?;
            get_value_at_path(&config, key)
        } else if local {
            let config = load_config_value(config_path_local()?)?;
            get_value_at_path(&config, key)
        } else {
            let local_config = load_config_value(config_path_local_optional())?;
            if let Some(value) = get_value_at_path(&local_config, key) {
                Some(value)
            } else {
                let global_config = load_config_value(config_path_global()?)?;
                get_value_at_path(&global_config, key)
            }
        };

        match value {
            Some(value) => {
                println!("{}", format_value(&value));
                Ok(())
            }
            None => Err(BucketError::NotFound(format!(
                "Key '{}' not found in configuration",
                key
            ))),
        }
    }

    fn set_value(&self, command: &ConfigSetCommand) -> Result<(), BucketError> {
        let scope_path = match (command.global, command.local) {
            (true, false) => config_path_global()?,
            (false, true) => config_path_local()?,
            _ => {
                return Err(BucketError::InvalidData(
                    "Specify --global or --local when setting a value".to_string(),
                ))
            }
        };

        let mut config = load_config_value(scope_path.clone())?;
        let value = parse_value(&command.value);
        set_value_at_path(&mut config, &command.key, value)?;
        save_config_value(scope_path, &config)?;
        Ok(())
    }

    fn unset_value(&self, key: &str, global: bool, local: bool) -> Result<(), BucketError> {
        let scope_path = match (global, local) {
            (true, false) => config_path_global()?,
            (false, true) => config_path_local()?,
            _ => {
                return Err(BucketError::InvalidData(
                    "Specify --global or --local when unsetting a value".to_string(),
                ))
            }
        };

        let mut config = load_config_value(scope_path.clone())?;
        if remove_value_at_path(&mut config, key) {
            save_config_value(scope_path, &config)?;
            Ok(())
        } else {
            Err(BucketError::NotFound(format!(
                "Key '{}' not found in configuration",
                key
            )))
        }
    }

    fn list_values(&self, command: &ConfigListCommand) -> Result<(), BucketError> {
        let config = if command.global {
            load_config_value(config_path_global()?)?
        } else if command.local {
            load_config_value(config_path_local()?)?
        } else if command.effective {
            let global_config = load_config_value(config_path_global()?)?;
            let local_config = load_config_value(config_path_local_optional())?;
            merge_values(global_config, &local_config)
        } else {
            let local_config = load_config_value(config_path_local_optional())?;
            if is_empty_table(&local_config) {
                load_config_value(config_path_global()?)?
            } else {
                local_config
            }
        };

        if is_empty_table(&config) {
            println!("{}", EMPTY_CONFIG_MESSAGE);
        } else {
            println!("{}", toml::to_string_pretty(&config).unwrap_or_default());
        }

        Ok(())
    }
}

fn config_path_global() -> Result<PathBuf, BucketError> {
    GlobalConfig::config_path()
}

fn config_path_local() -> Result<PathBuf, BucketError> {
    let current_dir = CURRENT_DIR.with(|dir| dir.clone());
    let buckets_repo_path =
        find_directory_in_parents(&current_dir, ".buckets").ok_or(BucketError::NotInRepo)?;
    Ok(buckets_repo_path.join("config"))
}

fn config_path_local_optional() -> ConfigPath {
    config_path_local().into()
}

fn load_config_value(path: impl Into<ConfigPath>) -> Result<Value, BucketError> {
    let path = path.into();
    match path {
        ConfigPath::Known(path) => load_config_value_from_path(&path),
        ConfigPath::Missing => Ok(Value::Table(toml::map::Map::new())),
    }
}

fn load_config_value_from_path(path: &PathBuf) -> Result<Value, BucketError> {
    if !path.exists() {
        return Ok(Value::Table(toml::map::Map::new()));
    }

    let content = fs::read_to_string(path)?;
    toml::from_str(&content).map_err(|e| {
        BucketError::InvalidData(format!("Failed to parse config: {}", e))
    })
}

fn save_config_value(path: PathBuf, value: &Value) -> Result<(), BucketError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(value).map_err(|e| {
        BucketError::InvalidData(format!("Failed to serialize config: {}", e))
    })?;
    fs::write(path, content)?;
    Ok(())
}

fn set_value_at_path(root: &mut Value, key: &str, value: Value) -> Result<(), BucketError> {
    let mut current = root
        .as_table_mut()
        .ok_or_else(|| BucketError::InvalidData("Configuration root is not a table".to_string()))?;

    let parts: Vec<&str> = key.split('.').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return Err(BucketError::InvalidData(
            "Configuration key cannot be empty".to_string(),
        ));
    }

    let mut value = Some(value);

    for (index, part) in parts.iter().enumerate() {
        let is_last = index == parts.len() - 1;
        if is_last {
            current.insert(
                part.to_string(),
                value
                    .take()
                    .expect("Internal error: value was already consumed before reaching final key"),
            );
        } else {
            let entry = current
                .entry(part.to_string())
                .or_insert_with(|| Value::Table(toml::map::Map::new()));
            current = entry.as_table_mut().ok_or_else(|| {
                BucketError::InvalidData(format!(
                    "Configuration key '{}' conflicts with a non-table value",
                    part
                ))
            })?;
        }
    }

    Ok(())
}

fn get_value_at_path(root: &Value, key: &str) -> Option<Value> {
    let parts: Vec<&str> = key.split('.').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut current = root;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current.clone())
}

fn remove_value_at_path(root: &mut Value, key: &str) -> bool {
    let parts: Vec<&str> = key.split('.').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
        return false;
    }

    let mut current = match root.as_table_mut() {
        Some(table) => table,
        None => return false,
    };

    for (index, part) in parts.iter().enumerate() {
        let is_last = index == parts.len() - 1;
        if is_last {
            return current.remove(*part).is_some();
        }

        match current.get_mut(*part) {
            Some(Value::Table(table)) => current = table,
            _ => return false,
        }
    }

    false
}

fn merge_values(mut base: Value, overlay: &Value) -> Value {
    match (&mut base, overlay) {
        (Value::Table(base_table), Value::Table(overlay_table)) => {
            for (key, overlay_value) in overlay_table {
                match base_table.get_mut(key) {
                    Some(base_value) => {
                        let merged = merge_values(base_value.clone(), overlay_value);
                        *base_value = merged;
                    }
                    None => {
                        base_table.insert(key.clone(), overlay_value.clone());
                    }
                }
            }
            base
        }
        _ => overlay.clone(),
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

fn parse_value(raw: &str) -> Value {
    if raw.eq_ignore_ascii_case("true") {
        return Value::Boolean(true);
    }
    if raw.eq_ignore_ascii_case("false") {
        return Value::Boolean(false);
    }

    if let Ok(integer) = raw.parse::<i64>() {
        return Value::Integer(integer);
    }

    if let Ok(float) = raw.parse::<f64>() {
        return Value::Float(float);
    }

    Value::String(raw.to_string())
}

fn is_empty_table(value: &Value) -> bool {
    matches!(value, Value::Table(table) if table.is_empty())
}

enum ConfigPath {
    Known(PathBuf),
    Missing,
}

impl From<PathBuf> for ConfigPath {
    fn from(path: PathBuf) -> Self {
        ConfigPath::Known(path)
    }
}

impl From<Result<PathBuf, BucketError>> for ConfigPath {
    fn from(result: Result<PathBuf, BucketError>) -> Self {
        match result {
            Ok(path) => ConfigPath::Known(path),
            Err(_) => ConfigPath::Missing,
        }
    }
}
