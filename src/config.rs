#[derive(serde::Serialize)]
pub(crate) struct Config {
    pub(crate) network: NetworkConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) database: Option<DatabaseConfig>,
}

#[derive(serde::Serialize)]
pub(crate) struct NetworkConfig {
    pub(crate) ntp_server: String,
    pub(crate) ip_check: String,
    pub(crate) url_check: String,
}

#[derive(serde::Serialize)]
pub(crate) struct DatabaseConfig {
    pub(crate) postgresql_connection: String,
}
