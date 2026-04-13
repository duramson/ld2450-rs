use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_device")]
    pub device: PathBuf,
    #[serde(default = "default_baud_rate")]
    pub baud_rate: u32,
    #[serde(default = "default_socket_path")]
    pub socket_path: PathBuf,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_device() -> PathBuf {
    PathBuf::from("/dev/ttyAMA0")
}

fn default_baud_rate() -> u32 {
    256000
}

fn default_socket_path() -> PathBuf {
    PathBuf::from("/run/ld2450/radar.sock")
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device: default_device(),
            baud_rate: default_baud_rate(),
            socket_path: default_socket_path(),
            log_level: default_log_level(),
        }
    }
}

impl Config {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
