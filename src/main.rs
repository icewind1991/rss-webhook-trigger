mod config;
mod error;
mod fetcher;
mod hub;

use crate::config::{Config, FeedConfig};
use crate::error::{FetchError, FetchFeedError, HubError, ParseFeedError};
use crate::fetcher::{next_fetch, CacheHeaders, FetchPlan, FetchResponse};
use main_error::MainResult;
use reqwest::{Client, Response, StatusCode};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::future::ready;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::time::{Duration};
use reqwest::header::{HeaderValue, USER_AGENT};
use syndication::Feed;
use tokio::select;
use tokio::signal::ctrl_c;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};

const FETCHER_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"), " (", env!("CARGO_PKG_REPOSITORY"), ")");

#[tokio::main]
async fn main() -> MainResult {
    tracing_subscriber::fmt::init();
    let mut args = std::env::args();
    let bin = args.next().unwrap();

    let file = match args.next() {
        Some(file) => file,
        None => {
            eprintln!("Usage {} <config file>", bin);
            return Ok(());
        }
    };

    let config = Config::from_file(&file)?;

    info!("Running rss trigger for {} feeds", config.feed.len());

    let ctrl_c = async {
        ctrl_c().await.ok();
    };

    select! {
        _ = ctrl_c => {},
        _ = main_loop(config) => {}
    }
    Ok(())
}

async fn main_loop(config: Config) {
    let mut fetcher = FeedFetcher::new(config.interval());

    loop {
        for feed in config.feed.iter() {
            match fetcher.check_feed_updated(&feed.feed).await {
                Ok(true) => {
                    trigger(&fetcher.client, feed).await;
                }
                Err(e) => error!(error = ?e, feed = feed.feed, "failed to check feed"),
                Ok(false) => {}
            }
        }

        sleep(config.interval()).await;
    }
}

#[instrument(skip_all, fields(feed = feed.feed))]
async fn trigger(client: &Client, feed: &FeedConfig) {
    info!("Triggering hook");
    let mut req = client
        .post(&feed.hook)
        .header("user-agent", "rss-webhook-trigger");
    for (key, value) in &feed.headers {
        req = req.header(key, value);
    }
    if !feed.body.is_null() {
        req = req.json(&feed.body);
    }
    debug!(request = ?req, "sending trigger request");
    if let Err(e) = req.send().await.and_then(|res| res.error_for_status()) {
        error!("{:#}", e);
    }
}

pub struct FeedFetcher {
    client: Client,
    base_interval: Duration,
    cache: HashMap<String, u64>,
    fetch_plans: HashMap<String, FetchPlan>,
}

impl FeedFetcher {
    pub fn new(interval: Duration) -> Self {
        FeedFetcher {
            client: Client::default(),
            base_interval: interval,
            cache: HashMap::default(),
            fetch_plans: HashMap::default(),
        }
    }

    pub fn should_update(&self, feed: &str) -> bool {
        match self.fetch_plans.get(feed) {
            Some(plan) => plan.is_elapsed(),
            None => true,
        }
    }

    #[instrument(skip(self))]
    pub async fn check_feed_updated(&mut self, feed: &str) -> Result<bool, FetchError> {
        if !self.should_update(feed) {
            warn!("skipping feed util rate limited expires");
            return Ok(false);
        }
        let plan = self.fetch_plans.remove(feed).unwrap_or_default();

        let fetch_result = self.get_feed_key(feed, &plan.headers).await;
        let new_key = match fetch_result.into_result() {
            Ok((new_key, new_plan)) => {
                self.fetch_plans.insert(feed.into(), next_fetch(self.base_interval, Some(new_plan)));
                new_key
            }
            Err((err, new_plan)) => {
                self.fetch_plans.insert(feed.into(), next_fetch(self.base_interval, Some(new_plan)));
                return Err(err);
            }
        };

        Ok(match (self.cache.get_mut(feed), new_key) {
            (Some(cached), Some(Some(new_key))) => {
                debug!(cached, new_key, "checked existing feed");
                if new_key != *cached {
                    *cached = new_key;
                    true
                } else {
                    false
                }
            }
            (None, Some(Some(new_key))) => {
                debug!(feed, "new feed");
                self.cache.insert(feed.into(), new_key);

                // don't trigger the actions on start
                false
            }
            (_, Some(None)) => {
                debug!("not modified response");
                false
            }
            (_, None) => {
                warn!("rate limited by server");
                false
            }
        })
    }

    #[instrument(skip(self))]
    async fn get_feed_key(
        &self,
        feed: &str,
        cache_headers: &CacheHeaders,
    ) -> FetchResponse<Option<u64>, FetchError> {
        if let Some(hub) = feed.strip_prefix("docker-hub://") {
            if let Some((user, repo)) = hub.split_once('/') {
                hub::tags(&self.client, user, repo, cache_headers)
                    .await
                    .map(|tags| {
                        if tags.is_empty() {
                            return ready(None);
                        }
                        let mut hasher = DefaultHasher::new();
                        for tag in tags {
                            tag.id.hash(&mut hasher);
                            tag.last_updated.hash(&mut hasher);
                        }
                        ready(Some(hasher.finish()))
                    }).await
                    .map_err(FetchError::Hub)
            } else {
                FetchResponse::Error {
                    error: HubError::InvalidFormat.into(),
                    headers: CacheHeaders::default(),
                }
            }
        } else {
            self.get_rss_feed_key(feed, cache_headers)
                .await
                .map_err(FetchError::Feed)
        }
    }

    #[instrument(skip(self))]
    async fn get_rss_feed_key(
        &self,
        feed: &str,
        cache_headers: &CacheHeaders,
    ) -> FetchResponse<Option<u64>, FetchFeedError> {
        let response = self
            .client
            .get(feed)
            .headers(cache_headers.headers())
            .header(USER_AGENT, HeaderValue::from_static(FETCHER_USER_AGENT))
            .send()
            .await;

        let plan_result = FetchResponse::from_result(response);
        plan_result
            .map_err(FetchFeedError::Network)
            .check_status_code(FetchFeedError::ClientError, FetchFeedError::ServerError)
            .map(parse_rss_response)
            .await
            .flatten()
    }
}

async fn parse_rss_response(response: Response) -> Result<Option<u64>, FetchFeedError> {
    if response.status() == StatusCode::NOT_MODIFIED {
        return Ok(None);
    }

    let content = response.text().await?;
    let channel = Feed::from_str(&content).map_err(ParseFeedError::Parse)?;

    let mut hasher = DefaultHasher::new();

    match channel {
        Feed::RSS(channel) => {
            let item = channel.items.first().ok_or(ParseFeedError::Empty)?;

            if let Some(guid) = item.guid() {
                guid.value.hash(&mut hasher);
            } else if let Some(date) = item.pub_date() {
                date.hash(&mut hasher);
            } else if let Some(link) = item.link() {
                link.hash(&mut hasher);
            } else {
                return Err(ParseFeedError::MissingKey.into());
            }
        }
        Feed::Atom(channel) => {
            let item = channel.entries().first().ok_or(ParseFeedError::Empty)?;
            item.id().hash(&mut hasher);
        }
    }

    Ok(Some(hasher.finish()))
}
