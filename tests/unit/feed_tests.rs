use std::collections::BTreeMap;

use hn_scored::{
    feed,
    time::Timestamp,
    types::{FeedFormat, LinkKind, State, Story},
};

#[test]
fn rss_empty_feed_uses_epoch_timestamp() {
    let view = feed::common::build_view(
        &State::empty(),
        1000,
        LinkKind::Article,
        FeedFormat::Rss,
        "https://hn.ysm.dev",
    );
    let rss = String::from_utf8(feed::rss::render(&view)).unwrap();
    assert!(
        rss.contains("Thu, 1 Jan 1970 00:00:00 +0000")
            || rss.contains("Thu, 01 Jan 1970 00:00:00 +0000")
    );
    assert!(rss.contains("<title>Hacker News - 1000+ points</title>"));
}

#[test]
fn json_feed_swaps_comment_urls() {
    let mut state = State::empty();
    state.stories.insert(1, story("", 0));
    let view = feed::common::build_view(
        &state,
        0,
        LinkKind::Comments,
        FeedFormat::Json,
        "https://hn.ysm.dev",
    );
    let json = String::from_utf8(feed::json_feed::render(&view)).unwrap();
    assert!(json.contains("\"url\": \"https://news.ycombinator.com/item?id=1\""));
    assert!(json.contains("\"external_url\": \"https://news.ycombinator.com/item?id=1\""));
}

#[test]
fn build_view_orders_by_threshold_then_id_desc() {
    let mut state = State::empty();
    state
        .stories
        .insert(10, story("https://example.com/10", 100));
    state
        .stories
        .insert(20, story("https://example.com/20", 100));
    let view = feed::common::build_view(
        &state,
        100,
        LinkKind::Article,
        FeedFormat::Rss,
        "https://hn.ysm.dev",
    );
    assert_eq!(
        view.items[0].comments_url,
        "https://news.ycombinator.com/item?id=20"
    );
    assert_eq!(
        view.items[1].comments_url,
        "https://news.ycombinator.com/item?id=10"
    );
}

fn story(url: &str, threshold: u16) -> Story {
    Story {
        id: if url.is_empty() {
            1
        } else {
            url.rsplit('/').next().unwrap().parse().unwrap()
        },
        title: "Story".to_string(),
        url: url.to_string(),
        hn_url: format!(
            "https://news.ycombinator.com/item?id={}",
            if url.is_empty() {
                1
            } else {
                url.rsplit('/').next().unwrap().parse::<u64>().unwrap()
            }
        ),
        score: 120,
        max_score: 120,
        comments: 3,
        by: "alice".to_string(),
        first_seen: Timestamp::parse("2025-04-14T10:00:00Z").unwrap(),
        story_time: 1_700_000_000,
        last_output_change_at: Timestamp::parse("2025-04-14T11:00:00Z").unwrap(),
        thresholds: BTreeMap::from([(
            threshold,
            Timestamp::parse("2025-04-14T10:00:00Z").unwrap(),
        )]),
    }
}
