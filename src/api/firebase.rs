use std::time::Duration;

use futures::{StreamExt, stream};
use rand::Rng;
use reqwest::header::CACHE_CONTROL;
use serde::de::DeserializeOwned;

use crate::{
    config::{FETCH_CONCURRENCY, FETCH_RETRIES},
    types::ApiItem,
};

#[derive(Clone, Debug)]
pub struct FirebaseClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Clone, Debug)]
pub struct DiscoveryResult {
    pub endpoint: &'static str,
    pub ids: Result<Vec<u64>, String>,
}

#[derive(Clone, Debug)]
pub struct ItemFetchResult {
    pub id: u64,
    pub item: Result<ApiItem, String>,
}

impl FirebaseClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn fetch_discovery(&self) -> Vec<DiscoveryResult> {
        let requests = [
            ("topstories", "/v0/topstories.json"),
            ("beststories", "/v0/beststories.json"),
            ("newstories", "/v0/newstories.json"),
        ];
        stream::iter(requests)
            .map(|(name, path)| async move {
                let ids = self.fetch_with_retry(path, name, false).await;
                DiscoveryResult {
                    endpoint: name,
                    ids,
                }
            })
            .buffer_unordered(requests.len())
            .collect()
            .await
    }

    pub async fn fetch_items(&self, ids: Vec<u64>) -> Vec<ItemFetchResult> {
        stream::iter(ids)
            .map(|id| async move {
                let path = format!("/v0/item/{id}.json");
                let item = self
                    .fetch_with_retry(&path, &format!("item {id}"), true)
                    .await;
                ItemFetchResult { id, item }
            })
            .buffer_unordered(FETCH_CONCURRENCY)
            .collect()
            .await
    }

    async fn fetch_with_retry<T>(&self, path: &str, label: &str, warn: bool) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let mut last_error = String::new();
        for attempt in 1..=FETCH_RETRIES {
            if attempt > 1 {
                tokio::time::sleep(retry_delay(attempt)).await;
            }
            match self.fetch_json(path).await {
                Ok(value) => return Ok(value),
                Err(error) => {
                    last_error = error;
                    if warn {
                        eprintln!(
                            "[WARN] fetch failed: {label} (attempt {attempt}/{FETCH_RETRIES}): {last_error}"
                        );
                    }
                }
            }
        }
        Err(last_error)
    }

    async fn fetch_json<T>(&self, path: &str) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .get(url)
            .header(CACHE_CONTROL, "max-age=60")
            .send()
            .await
            .map_err(|error| error.to_string())?;
        response
            .error_for_status()
            .map_err(|error| error.to_string())?
            .json()
            .await
            .map_err(|error| error.to_string())
    }
}

fn retry_delay(attempt: usize) -> Duration {
    if attempt <= 1 {
        return Duration::from_millis(0);
    }
    let base = match attempt {
        2 => 500_u64,
        _ => 1_000_u64,
    };
    let jitter = rand::thread_rng().gen_range((base / 2)..=(base + base / 2));
    Duration::from_millis(jitter.min(2_000))
}
