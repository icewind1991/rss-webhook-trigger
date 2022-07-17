use color_eyre::eyre::WrapErr;
use color_eyre::{eyre::ensure, Result};
use reqwest::Client;
use serde::Deserialize;
use time::OffsetDateTime;

pub async fn tags(client: &Client, user: &str, repo: &str) -> Result<Vec<HubTag>> {
    let result = client
        .get(format!(
            "https://hub.docker.com/v2/repositories/{}/{}/tags",
            user, repo
        ))
        .send()
        .await
        .wrap_err("error with sending docker hub request")?;
    ensure!(
        !result.status().is_client_error(),
        "error with sending docker hub request"
    );
    ensure!(
        !result.status().is_server_error(),
        "docker hub request returned an error"
    );
    Ok(result
        .json::<HubTagResponse>()
        .await
        .wrap_err("failed to parse hub response")?
        .results)
}

#[derive(Debug, Deserialize)]
pub struct HubTagResponse {
    results: Vec<HubTag>,
}

#[derive(Debug, Deserialize)]
pub struct HubTag {
    pub id: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub last_updated: OffsetDateTime,
    pub name: String,
}
