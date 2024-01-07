use std::fs::File;
use std::io::Write;
use std::path::{Path};
use serde::Serialize;
use toml::to_string;
#[derive(Serialize)]
struct Config {
    pub ntp_server: String,
    pub ip_check: String,
    url_check: String,
}

impl Config {
    fn default() -> Self {
        Config {
            ntp_server: "pool.ntp.org".to_string(),
            ip_check: "8.8.8.8".to_string(),
            url_check: "https://api.ipify.org".to_string(),
        }
    }
}

pub fn create_default_config(file_path: &Path) {
    println!("Creating config files in {:?}", file_path.as_os_str());

    let config = Config::default();
    let toml_string = to_string(&config).unwrap();
    let mut file = File::create(file_path.join("config")).unwrap();
    file.write_all(toml_string.as_bytes()).unwrap();

}
