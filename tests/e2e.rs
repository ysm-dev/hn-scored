use std::fs;

use hn_scored::{app, config::AppConfig};

#[tokio::test]
#[ignore = "hits the real Hacker News API"]
async fn real_hn_cycle_generates_all_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let config = AppConfig {
        state_path: dir.path().join("state.json"),
        output_dir: dir.path().join("dist"),
        base_url: "https://hn.ysm.dev".to_string(),
        api_base_url: hn_scored::config::BASE_DISCOVERY_URL.to_string(),
    };
    let code = app::run_once(
        &config,
        hn_scored::time::Timestamp::parse("2025-04-14T12:00:00Z").unwrap(),
        None,
    )
    .await
    .unwrap();
    assert!(matches!(code, 0 | 2));
    assert!(
        fs::read_to_string(config.output_dir.join("feeds/article/0.xml"))
            .unwrap()
            .contains("<rss")
    );
    assert!(
        fs::read_to_string(config.state_path)
            .unwrap()
            .contains("\"stories\"")
    );
}
