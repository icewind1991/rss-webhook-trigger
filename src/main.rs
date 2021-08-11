mod config;

use crate::config::Config;
use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use reqwest::Client;
use rss::Channel;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
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
    let mut fetcher = FeedFetcher::default();

    println!("Running rss trigger for {} feeds", config.feed.len());

    loop {
        for feed in config.feed.iter() {
            match fetcher.is_feed_updated(&feed.feed).await {
                Ok(true) => {
                    println!("Triggering hook for {}", feed.feed);
                    let mut req = fetcher.client.post(&feed.hook);
                    for (key, value) in &feed.headers {
                        req = req.header(key, value);
                    }
                    if !feed.body.is_null() {
                        req = req.json(&feed.body);
                    }
                    req.send().await?;
                }
                Err(e) => eprintln!("{:#}", e),
                Ok(false) => {}
            }
        }

        sleep(config.interval()).await;
    }
}

#[derive(Default)]
pub struct FeedFetcher {
    client: Client,
    cache: HashMap<String, u64>,
}

impl FeedFetcher {
    pub async fn is_feed_updated(&mut self, feed: &str) -> Result<bool> {
        let new_key = self.get_feed_key(feed).await?;

        Ok(match self.cache.get_mut(feed) {
            Some(cached) => {
                if *cached != new_key {
                    *cached = new_key;
                    true
                } else {
                    false
                }
            }
            None => {
                self.cache.insert(feed.into(), new_key);

                // dont trigger the actions on start

                false
            }
        })
    }

    async fn get_feed_key(&self, feed: &str) -> Result<u64> {
        let content = self
            .client
            .get(feed)
            .send()
            .await
            .wrap_err_with(|| eyre!("Failed to load feed {}", feed))?
            .bytes()
            .await
            .wrap_err_with(|| eyre!("Failed to load feed {}", feed))?;
        let channel = Channel::read_from(content.as_ref())
            .wrap_err_with(|| eyre!("Failed to parse feed {}", feed))?;
        let item = channel.items.first().ok_or(eyre!("Empty feed"))?;

        let mut hasher = DefaultHasher::new();
        if let Some(guid) = item.guid() {
            guid.value.hash(&mut hasher);
        } else if let Some(date) = item.pub_date() {
            date.hash(&mut hasher);
        } else if let Some(link) = item.link() {
            link.hash(&mut hasher);
        } else {
            return Err(eyre!("No guid, pubDate or link set on feed item"));
        }

        Ok(hasher.finish())
    }
}
