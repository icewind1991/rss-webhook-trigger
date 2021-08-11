use color_eyre::{eyre::WrapErr, Result};
use serde::Deserialize;
use std::fs::read_to_string;
use tokio::time::Duration;
use std::collections::HashMap;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Config {
    interval: Option<u64>,
    pub feed: Vec<FeedConfig>,
}

#[derive(Debug, Deserialize)]
pub struct FeedConfig {
    pub feed: String,
    pub hook: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Value,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let file = read_to_string(path)
            .wrap_err_with(|| format!("Failed to open config file {}", path))?;
        toml::from_str(&file).wrap_err_with(|| format!("Failed to open config file {}", path))
    }

    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.interval.unwrap_or(30 * 60))
    }
}
