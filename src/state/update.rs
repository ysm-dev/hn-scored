use std::collections::BTreeMap;

use crate::{
    state::threshold,
    time::Timestamp,
    types::{ApiItem, Story},
};

#[derive(Debug, Default)]
pub struct UpdateStats {
    pub created: bool,
    pub removed: bool,
    pub dead_removed: bool,
    pub crossings: usize,
    pub changed: bool,
    pub observed_score: Option<i64>,
}

pub fn apply_item(
    existing: Option<&Story>,
    previous_max_score: Option<i64>,
    item: &ApiItem,
    cycle_time: &Timestamp,
) -> (Option<Story>, UpdateStats) {
    if item.item_type.is_none() || item.title.is_none() {
        return (existing.cloned(), UpdateStats::default());
    }
    if item.item_type.as_deref() != Some("story") {
        let removed = existing.is_some();
        return (
            None,
            UpdateStats {
                removed,
                changed: removed,
                ..UpdateStats::default()
            },
        );
    }
    if item.dead.unwrap_or(false) || item.deleted.unwrap_or(false) {
        let removed = existing.is_some();
        return (
            None,
            UpdateStats {
                removed,
                dead_removed: removed,
                changed: removed,
                ..UpdateStats::default()
            },
        );
    }
    let normalized = normalize_item(item);
    match existing {
        Some(story) => update_existing(story, previous_max_score, normalized, cycle_time),
        None => create_story(previous_max_score, normalized, cycle_time),
    }
}

fn normalize_item(item: &ApiItem) -> ApiItem {
    ApiItem {
        id: item.id,
        item_type: Some("story".to_string()),
        title: item
            .title
            .as_ref()
            .map(|value| crate::util::decode_title(value)),
        url: Some(item.url.clone().unwrap_or_default()),
        score: Some(item.score.unwrap_or(0)),
        dead: Some(item.dead.unwrap_or(false)),
        deleted: Some(item.deleted.unwrap_or(false)),
        descendants: Some(item.descendants.unwrap_or(0)),
        by: Some(item.by.clone().unwrap_or_default()),
        time: Some(item.time.unwrap_or(0)),
    }
}

fn create_story(
    previous_max_score: Option<i64>,
    item: ApiItem,
    cycle_time: &Timestamp,
) -> (Option<Story>, UpdateStats) {
    let score = item.score.unwrap_or(0);
    let mut thresholds = BTreeMap::new();
    let crossings =
        threshold::record_crossings(&mut thresholds, previous_max_score, score, cycle_time);
    if thresholds.is_empty() {
        return (
            None,
            UpdateStats {
                changed: previous_max_score.is_none_or(|previous| score > previous),
                observed_score: Some(score),
                ..UpdateStats::default()
            },
        );
    }
    let story = Story {
        id: item.id,
        title: item.title.unwrap_or_default(),
        url: item.url.unwrap_or_default(),
        hn_url: crate::util::hn_item_url(item.id),
        score,
        max_score: previous_max_score.unwrap_or(score).max(score),
        comments: item.descendants.unwrap_or(0),
        by: item.by.unwrap_or_default(),
        first_seen: cycle_time.clone(),
        story_time: item.time.unwrap_or(0),
        last_output_change_at: cycle_time.clone(),
        thresholds,
    };
    (
        Some(story),
        UpdateStats {
            created: true,
            crossings,
            changed: true,
            observed_score: Some(score),
            ..UpdateStats::default()
        },
    )
}

fn update_existing(
    story: &Story,
    previous_max_score: Option<i64>,
    item: ApiItem,
    cycle_time: &Timestamp,
) -> (Option<Story>, UpdateStats) {
    let mut next = story.clone();
    let mut changed = false;
    let title = item.title.unwrap_or_default();
    let url = item.url.unwrap_or_default();
    let score = item.score.unwrap_or(0);
    let comments = item.descendants.unwrap_or(0);
    let by = item.by.unwrap_or_default();
    let story_time = item.time.unwrap_or(0);
    changed |= replace_if_needed(&mut next.title, title);
    changed |= replace_if_needed(&mut next.url, url);
    changed |= replace_if_needed(&mut next.score, score);
    changed |= replace_if_needed(&mut next.comments, comments);
    changed |= replace_if_needed(&mut next.by, by);
    changed |= replace_if_needed(&mut next.story_time, story_time);
    if score > next.max_score {
        next.max_score = score;
        changed = true;
    }
    let crossings =
        threshold::record_crossings(&mut next.thresholds, previous_max_score, score, cycle_time);
    changed |= crossings > 0;
    if changed {
        next.last_output_change_at = cycle_time.clone();
    }
    (
        Some(next),
        UpdateStats {
            crossings,
            changed,
            observed_score: Some(score),
            ..UpdateStats::default()
        },
    )
}

fn replace_if_needed<T: PartialEq>(target: &mut T, next: T) -> bool {
    if *target == next {
        false
    } else {
        *target = next;
        true
    }
}
