use std::{
    fs,
    time::{Duration, Instant},
};

use axum::http::StatusCode;
use hn_scored::{api::firebase::FirebaseClient, app, config::AppConfig, state};

use crate::common::{MockHnServer, fixed_time, sample_item};

#[tokio::test]
async fn score_rise_and_drop_retains_threshold_feed_membership() {
    let server = MockHnServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let config = config(dir.path(), &server.base_url());
    server.set_lists(vec![1], vec![], vec![]).await;
    server.set_item(1, sample_item(1, 60, 5)).await;
    assert_eq!(
        app::run_once(&config, fixed_time("2025-04-14T12:00:00Z"), None)
            .await
            .unwrap(),
        0
    );
    server.set_lists(vec![], vec![], vec![]).await;
    server.set_item(1, sample_item(1, 120, 7)).await;
    assert_eq!(
        app::run_once(&config, fixed_time("2025-04-14T12:01:00Z"), None)
            .await
            .unwrap(),
        0
    );
    server.set_item(1, sample_item(1, 80, 9)).await;
    assert_eq!(
        app::run_once(&config, fixed_time("2025-04-14T12:02:00Z"), None)
            .await
            .unwrap(),
        0
    );
    let state = state::store::load_state(&config.state_path).unwrap();
    let story = state.stories.get(&1).unwrap();
    assert_eq!(story.score, 80);
    assert_eq!(story.max_score, 120);
    assert_eq!(
        story.thresholds.get(&100).unwrap().iso8601(),
        "2025-04-14T12:01:00Z"
    );
    let rss = fs::read_to_string(config.output_dir.join("feeds/article/100.xml")).unwrap();
    assert!(rss.contains("https://news.ycombinator.com/item?id=1"));
}

#[tokio::test]
async fn dead_story_is_removed_from_state_and_feeds() {
    let server = MockHnServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let config = config(dir.path(), &server.base_url());
    server.set_lists(vec![2], vec![], vec![]).await;
    server.set_item(2, sample_item(2, 150, 12)).await;
    app::run_once(&config, fixed_time("2025-04-14T12:00:00Z"), None)
        .await
        .unwrap();
    server
        .set_item(
            2,
            serde_json::json!({"id":2,"type":"story","title":"Story 2","dead":true}),
        )
        .await;
    app::run_once(&config, fixed_time("2025-04-14T12:01:00Z"), None)
        .await
        .unwrap();
    let state = state::store::load_state(&config.state_path).unwrap();
    assert!(state.stories.is_empty());
    let rss = fs::read_to_string(config.output_dir.join("feeds/article/100.xml")).unwrap();
    assert!(!rss.contains("item?id=2"));
}

#[tokio::test]
async fn failed_item_fetch_keeps_existing_story() {
    let server = MockHnServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let config = config(dir.path(), &server.base_url());
    server.set_lists(vec![3], vec![], vec![]).await;
    server.set_item(3, sample_item(3, 80, 4)).await;
    app::run_once(&config, fixed_time("2025-04-14T12:00:00Z"), None)
        .await
        .unwrap();
    let before = fs::read(&config.state_path).unwrap();
    server.set_lists(vec![], vec![], vec![]).await;
    server.fail_item(3, StatusCode::INTERNAL_SERVER_ERROR).await;
    app::run_once(&config, fixed_time("2025-04-14T12:01:00Z"), None)
        .await
        .unwrap();
    assert_eq!(fs::read(&config.state_path).unwrap(), before);
}

#[tokio::test]
async fn full_discovery_outage_is_fatal_and_preserves_bytes() {
    let server = MockHnServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let config = config(dir.path(), &server.base_url());
    fs::write(&config.state_path, b"{\n  \"version\": 1,\n  \"last_output_change_at\": \"1970-01-01T00:00:00Z\",\n  \"stories\": {}\n}\n").unwrap();
    fs::create_dir_all(config.output_dir.join("feeds/article")).unwrap();
    fs::create_dir_all(config.output_dir.join("feeds/comments")).unwrap();
    fs::write(config.output_dir.join("_headers"), b"keep\n").unwrap();
    fs::write(config.output_dir.join("index.html"), b"keep\n").unwrap();
    server
        .fail_lists(
            StatusCode::BAD_GATEWAY,
            StatusCode::BAD_GATEWAY,
            StatusCode::BAD_GATEWAY,
        )
        .await;
    let state_before = fs::read(&config.state_path).unwrap();
    let headers_before = fs::read(config.output_dir.join("_headers")).unwrap();
    assert_eq!(
        app::run_once(&config, fixed_time("2025-04-14T12:00:00Z"), None)
            .await
            .unwrap(),
        1
    );
    assert_eq!(fs::read(&config.state_path).unwrap(), state_before);
    assert_eq!(
        fs::read(config.output_dir.join("_headers")).unwrap(),
        headers_before
    );
}

#[tokio::test]
async fn unchanged_cycle_returns_two_and_keeps_bytes_identical() {
    let server = MockHnServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let config = config(dir.path(), &server.base_url());
    server.set_lists(vec![4], vec![], vec![]).await;
    server.set_item(4, sample_item(4, 90, 6)).await;
    assert_eq!(
        app::run_once(&config, fixed_time("2025-04-14T12:00:00Z"), None)
            .await
            .unwrap(),
        0
    );
    let state_before = fs::read(&config.state_path).unwrap();
    let feed_before = fs::read(config.output_dir.join("feeds/article/50.xml")).unwrap();
    server.set_lists(vec![], vec![], vec![]).await;
    server.set_item(4, sample_item(4, 90, 6)).await;
    let code = app::run_once(&config, fixed_time("2025-04-14T12:10:00Z"), None)
        .await
        .unwrap();
    let state_after = fs::read(&config.state_path).unwrap();
    let feed_after = fs::read(config.output_dir.join("feeds/article/50.xml")).unwrap();
    assert_eq!(
        code,
        2,
        "state_changed={}, feed_changed={}",
        state_after != state_before,
        feed_after != feed_before
    );
    assert_eq!(state_after, state_before);
    assert_eq!(feed_after, feed_before);
}

#[tokio::test]
async fn feed_caps_at_two_hundred_items_with_descending_id_tiebreak() {
    let server = MockHnServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let config = config(dir.path(), &server.base_url());
    let ids = (1..=205).collect::<Vec<_>>();
    server.set_lists(ids.clone(), vec![], vec![]).await;
    for id in ids {
        server.set_item(id, sample_item(id, 0, 0)).await;
    }
    app::run_once(&config, fixed_time("2025-04-14T12:00:00Z"), None)
        .await
        .unwrap();
    let rss = fs::read_to_string(config.output_dir.join("feeds/article/0.xml")).unwrap();
    assert!(rss.contains("item?id=205"));
    assert!(!rss.contains("item?id=5</guid>"));
}

#[tokio::test]
async fn firebase_requests_send_cache_control_max_age_sixty() {
    let server = MockHnServer::start().await;
    server.set_lists(vec![7], vec![], vec![]).await;
    server.set_item(7, sample_item(7, 10, 1)).await;
    let client = FirebaseClient::new(&server.base_url());
    let discovery = client.fetch_discovery().await;
    assert_eq!(
        discovery
            .iter()
            .filter(|entry| entry.endpoint == "topstories")
            .count(),
        1
    );
    let _ = client.fetch_items(vec![7]).await;
    assert!(
        server
            .recorded_cache_control_values()
            .await
            .iter()
            .all(|value| value == "max-age=60")
    );
}

#[tokio::test]
async fn discovery_fetches_run_in_parallel() {
    let server = MockHnServer::start().await;
    server.set_lists(vec![1], vec![2], vec![3]).await;
    server.set_list_delay_ms(250).await;
    let client = FirebaseClient::new(&server.base_url());
    let started = Instant::now();
    let _ = client.fetch_discovery().await;
    assert!(started.elapsed() < Duration::from_millis(550));
}

fn config(root: &std::path::Path, api_base_url: &str) -> AppConfig {
    AppConfig {
        state_path: root.join("state.json"),
        output_dir: root.join("dist"),
        base_url: "https://hn.ysm.dev".to_string(),
        api_base_url: api_base_url.to_string(),
    }
}
