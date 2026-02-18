use crate::postgres_db::TlsConfig;

#[derive(serde::Serialize)]
pub struct Config {
    pub database_type: String,
    pub network: NetworkConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseConfig>,
}

#[derive(serde::Serialize)]
pub struct NetworkConfig {
    pub ntp_server: String,
    pub ip_check: String,
    pub url_check: String,
}

#[derive(serde::Serialize)]
pub struct DatabaseConfig {
    pub postgresql_connection: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsConfig>,
}
