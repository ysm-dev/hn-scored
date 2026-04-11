use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{config::STATE_VERSION, time::Timestamp};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeedFormat {
    Rss,
    Atom,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkKind {
    Article,
    Comments,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Story {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub hn_url: String,
    pub score: i64,
    pub max_score: i64,
    pub comments: u64,
    pub by: String,
    pub first_seen: Timestamp,
    pub story_time: i64,
    pub last_output_change_at: Timestamp,
    pub thresholds: BTreeMap<u16, Timestamp>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct State {
    pub version: u64,
    pub last_output_change_at: Timestamp,
    pub stories: BTreeMap<u64, Story>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ApiItem {
    pub id: u64,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub score: Option<i64>,
    pub dead: Option<bool>,
    pub deleted: Option<bool>,
    pub descendants: Option<u64>,
    pub by: Option<String>,
    pub time: Option<i64>,
}

impl State {
    pub fn empty() -> Self {
        Self {
            version: STATE_VERSION,
            last_output_change_at: Timestamp::epoch(),
            stories: BTreeMap::new(),
        }
    }

    pub fn retained_ids(&self) -> Vec<u64> {
        self.stories.keys().copied().collect()
    }

    pub fn recompute_last_output_change_at(&mut self) {
        self.last_output_change_at = self
            .stories
            .values()
            .map(|story| story.last_output_change_at.clone())
            .max()
            .unwrap_or_else(Timestamp::epoch);
    }
}
