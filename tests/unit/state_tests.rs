use std::{collections::BTreeMap, fs};

use hn_scored::{
    state,
    time::Timestamp,
    types::{ApiItem, State, Story},
};

#[test]
fn threshold_zero_includes_negative_scores() {
    let crossings = state::threshold::crossed_thresholds(-5);
    assert_eq!(crossings, vec![0]);
}

#[test]
fn cleanup_removes_expired_thresholds_and_empty_stories() {
    let cycle = Timestamp::parse("2025-04-14T12:00:00Z").unwrap();
    let mut state = State::empty();
    state.stories.insert(
        1,
        story_with_thresholds([(0, "2025-04-07T12:00:00Z"), (50, "2025-04-06T11:59:59Z")]),
    );
    let changed = state::cleanup::cleanup_state(&mut state, &cycle);
    assert!(changed);
    assert_eq!(state.stories.get(&1).unwrap().thresholds.len(), 1);
    assert!(state.stories.get(&1).unwrap().thresholds.contains_key(&0));
}

#[test]
fn invalid_response_keeps_existing_story() {
    let cycle = Timestamp::parse("2025-04-14T12:00:00Z").unwrap();
    let existing = story_with_thresholds([(0, "2025-04-14T12:00:00Z")]);
    let item = ApiItem {
        id: 1,
        item_type: Some("story".to_string()),
        title: None,
        url: None,
        score: None,
        dead: None,
        deleted: None,
        descendants: None,
        by: None,
        time: None,
    };
    let (next, stats) =
        state::update::apply_item(Some(&existing), Some(existing.max_score), &item, &cycle);
    assert_eq!(next.unwrap(), existing);
    assert!(!stats.changed);
}

#[test]
fn state_store_recovers_from_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    fs::write(&path, b"{not-json").unwrap();
    let state = state::store::load_state(&path).unwrap();
    assert_eq!(state, State::empty());
}

#[test]
fn state_serialization_orders_numeric_keys() {
    let mut state = State::empty();
    state.stories.insert(
        10,
        story_with_thresholds([(100, "2025-04-14T12:00:00Z"), (50, "2025-04-14T11:00:00Z")]),
    );
    state
        .stories
        .insert(2, story_with_thresholds([(0, "2025-04-14T10:00:00Z")]));
    let text = String::from_utf8(state::store::serialize_state(&state).unwrap()).unwrap();
    assert!(text.find("\"2\": {").unwrap() < text.find("\"10\": {").unwrap());
    assert!(text.find("\"50\"").unwrap() < text.find("\"100\"").unwrap());
}

#[test]
fn state_load_derives_max_score_history_from_existing_stories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    let mut state = State::empty();
    state
        .stories
        .insert(1, story_with_thresholds([(100, "2025-04-14T12:00:00Z")]));
    let mut value = serde_json::to_value(&state).unwrap();
    value.as_object_mut().unwrap().remove("max_scores");
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();

    let loaded = state::store::load_state(&path).unwrap();

    assert_eq!(loaded.max_scores.get(&1), Some(&100));
}

#[test]
fn state_load_rejects_malformed_max_score_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    fs::write(
        &path,
        br#"{"version":1,"max_scores":{"1":"invalid"},"stories":{}}"#,
    )
    .unwrap();

    let error = state::store::load_state(&path).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("invalid state.json max_scores value")
    );
}

fn story_with_thresholds<const N: usize>(pairs: [(u16, &str); N]) -> Story {
    Story {
        id: 1,
        title: "Story 1".to_string(),
        url: "https://example.com/1".to_string(),
        hn_url: "https://news.ycombinator.com/item?id=1".to_string(),
        score: 100,
        max_score: 100,
        comments: 10,
        by: "alice".to_string(),
        first_seen: Timestamp::parse("2025-04-14T10:00:00Z").unwrap(),
        story_time: 1_700_000_000,
        last_output_change_at: Timestamp::parse("2025-04-14T10:00:00Z").unwrap(),
        thresholds: pairs
            .into_iter()
            .map(|(k, v)| (k, Timestamp::parse(v).unwrap()))
            .collect::<BTreeMap<_, _>>(),
    }
}
