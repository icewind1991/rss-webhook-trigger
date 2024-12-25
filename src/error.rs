use reqwest::StatusCode;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseFeedError {
    #[error("{0}")]
    Parse(<syndication::Feed as FromStr>::Err),
    #[error("Empty feed")]
    Empty,
    #[error("No guid, pubDate or link set on feed item")]
    MissingKey,
}

#[derive(Debug, Error)]
pub enum FetchFeedError {
    #[error("Error while fetching feed: {0:#}")]
    Network(#[from] reqwest::Error),
    #[error("Error while parsing feed: {0:#}")]
    Parse(#[from] ParseFeedError),
    #[error("Docker hub returned a server error {0}")]
    ServerError(StatusCode),
    #[error("Docker hub returned a client error {0}")]
    ClientError(StatusCode),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Error while reading config file {}: {:#}", path.display(), error)]
    Read {
        error: std::io::Error,
        path: PathBuf,
    },
    #[error("Error while parse config file {}: {:#}", path.display(), error)]
    Parse {
        error: toml::de::Error,
        path: PathBuf,
    },
}

#[derive(Debug, Error)]
pub enum HubError {
    #[error("Error while fetching docker hub info: {0:#}")]
    Network(#[from] reqwest::Error),
    #[error("Error while parsing hub response: {0:#}")]
    Parse(#[from] serde_json::Error),
    #[error("Docker hub returned a server error {0}")]
    ServerError(StatusCode),
    #[error("Docker hub returned a client error {0}")]
    ClientError(StatusCode),
    #[error("Invalid hub url format")]
    InvalidFormat,
}

#[derive(Debug, Error)]
pub enum FetchError {
    #[error(transparent)]
    Feed(#[from] FetchFeedError),
    #[error(transparent)]
    Hub(#[from] HubError),
}
