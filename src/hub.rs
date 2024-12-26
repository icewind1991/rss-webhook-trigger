use crate::error::HubError;
use crate::fetcher::{CacheHeaders, FetchResponse};
use reqwest::{Client, StatusCode};
use reqwest::header::{HeaderValue, USER_AGENT};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::instrument;
use crate::FETCHER_USER_AGENT;

#[instrument(skip(client))]
pub async fn tags(
    client: &Client,
    user: &str,
    repo: &str,
    cache_headers: &CacheHeaders,
) -> FetchResponse<Vec<HubTag>, HubError> {
    let result = client
        .get(dbg!(format!(
            "https://hub.docker.com/v2/repositories/{}/{}/tags",
            user, repo
        )))
        .headers(cache_headers.headers())
        .header(USER_AGENT, HeaderValue::from_static(FETCHER_USER_AGENT))
        .send()
        .await;

    FetchResponse::from_result(result)
        .map_err(HubError::Network)
        .check_status_code(HubError::ClientError, HubError::ServerError)
        .map(|response| async {
            if response.status() == StatusCode::NOT_MODIFIED {
                return Ok(Vec::new());
            }
            response
                .text()
                .await
                .map_err(HubError::Network)
                .and_then(|text| serde_json::from_str::<HubTagResponse>(&text).map_err(HubError::Parse))
                .map(|result| result.results)
        }).await.flatten()
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
}
