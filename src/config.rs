use std::path::PathBuf;

use crate::error::AppError;

pub const STATE_VERSION: u64 = 1;
pub const MAX_ITEMS_PER_FEED: usize = 200;
pub const RETENTION_DAYS: i64 = 7;
pub const FETCH_CONCURRENCY: usize = 50;
pub const FETCH_RETRIES: usize = 3;
pub const BASE_DISCOVERY_URL: &str = "https://hacker-news.firebaseio.com";
pub const HN_ITEM_URL_PREFIX: &str = "https://news.ycombinator.com/item?id=";
pub const HN_HOME_URL: &str = "https://news.ycombinator.com";
pub const REPOSITORY_URL: &str = "https://github.com/ysm-dev/hn-scored";
pub const THRESHOLDS: [u16; 16] = [
    0, 50, 100, 150, 200, 250, 300, 350, 400, 450, 500, 600, 700, 800, 900, 1000,
];

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub state_path: PathBuf,
    pub output_dir: PathBuf,
    pub base_url: String,
    pub api_base_url: String,
}

pub fn normalize_base_url(input: &str) -> Result<String, AppError> {
    let trimmed = input.strip_suffix('/').unwrap_or(input);
    let url = url::Url::parse(trimmed).map_err(|_| AppError::InvalidBaseUrl(input.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(AppError::InvalidBaseUrl(input.to_string()));
    }
    Ok(trimmed.to_string())
}
