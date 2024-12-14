use color_eyre::{eyre::WrapErr, Result};
use reqwest::header::{HeaderValue, InvalidHeaderValue};
use secretfile::{load, SecretError};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fs::read_to_string;
use std::path::Path;
use tokio::time::Duration;

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
    pub headers: HashMap<String, HeaderVal>,
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

#[derive(Debug)]
pub struct HeaderVal(String);

impl<'de> Deserialize<'de> for HeaderVal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let str = load_secret(raw).map_err(D::Error::custom)?;
        Ok(HeaderVal(str))
    }
}

impl TryFrom<&HeaderVal> for HeaderValue {
    type Error = InvalidHeaderValue;

    fn try_from(header: &HeaderVal) -> std::result::Result<Self, Self::Error> {
        header.0.as_str().try_into()
    }
}

fn load_secret(raw: String) -> Result<String, SecretError> {
    let path: &Path = raw.as_ref();
    if (raw.starts_with('/') && path.exists()) || raw.contains("$CREDENTIALS_DIRECTORY") {
        load(&raw)
    } else {
        Ok(raw)
    }
}
