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

#[test]
fn elapsed_suffix_appears_in_all_three_formats() {
    let mut state = State::empty();
    // story_time -> threshold crossing spans 2 days, 3 hours, 32 minutes.
    state.stories.insert(
        1,
        story_with_timing(1, 100, unix("2025-04-14T00:00:00Z"), "2025-04-16T03:32:00Z"),
    );
    let view = feed::common::build_view(
        &state,
        100,
        LinkKind::Article,
        FeedFormat::Rss,
        "https://hn.ysm.dev",
    );
    let rss = String::from_utf8(feed::rss::render(&view)).unwrap();
    let atom = String::from_utf8(feed::atom::render(&view)).unwrap();
    let json = String::from_utf8(feed::json_feed::render(&view)).unwrap();
    assert!(rss.contains("<title>Story (2d 3h 32m)</title>"));
    assert!(atom.contains("<title>Story (2d 3h 32m)</title>"));
    assert!(json.contains("\"title\": \"Story (2d 3h 32m)\""));
}

#[test]
fn elapsed_suffix_omits_days_when_under_one_day() {
    let mut state = State::empty();
    state.stories.insert(
        1,
        story_with_timing(1, 100, unix("2025-04-14T10:00:00Z"), "2025-04-14T13:32:00Z"),
    );
    let view = feed::common::build_view(
        &state,
        100,
        LinkKind::Article,
        FeedFormat::Rss,
        "https://hn.ysm.dev",
    );
    let rss = String::from_utf8(feed::rss::render(&view)).unwrap();
    assert!(rss.contains("<title>Story (3h 32m)</title>"));
}

#[test]
fn elapsed_suffix_shows_minutes_only_under_one_hour() {
    let mut state = State::empty();
    state.stories.insert(
        1,
        story_with_timing(1, 100, unix("2025-04-14T10:00:00Z"), "2025-04-14T10:32:00Z"),
    );
    let view = feed::common::build_view(
        &state,
        100,
        LinkKind::Article,
        FeedFormat::Rss,
        "https://hn.ysm.dev",
    );
    let rss = String::from_utf8(feed::rss::render(&view)).unwrap();
    assert!(rss.contains("<title>Story (32m)</title>"));
}

#[test]
fn elapsed_suffix_at_exactly_one_day_omits_hours_keeps_minutes() {
    let mut state = State::empty();
    state.stories.insert(
        1,
        story_with_timing(1, 100, unix("2025-04-14T00:00:00Z"), "2025-04-15T00:00:00Z"),
    );
    let view = feed::common::build_view(
        &state,
        100,
        LinkKind::Article,
        FeedFormat::Rss,
        "https://hn.ysm.dev",
    );
    let rss = String::from_utf8(feed::rss::render(&view)).unwrap();
    assert!(rss.contains("<title>Story (1d 0m)</title>"));
}

#[test]
fn elapsed_suffix_at_exactly_one_hour_shows_zero_minutes() {
    let mut state = State::empty();
    state.stories.insert(
        1,
        story_with_timing(1, 100, unix("2025-04-14T10:00:00Z"), "2025-04-14T11:00:00Z"),
    );
    let view = feed::common::build_view(
        &state,
        100,
        LinkKind::Article,
        FeedFormat::Rss,
        "https://hn.ysm.dev",
    );
    let rss = String::from_utf8(feed::rss::render(&view)).unwrap();
    assert!(rss.contains("<title>Story (1h 0m)</title>"));
}

#[test]
fn threshold_zero_feed_has_no_elapsed_suffix() {
    let mut state = State::empty();
    state.stories.insert(
        1,
        story_with_timing(1, 0, unix("2025-04-14T00:00:00Z"), "2025-04-16T03:32:00Z"),
    );
    let view = feed::common::build_view(
        &state,
        0,
        LinkKind::Article,
        FeedFormat::Rss,
        "https://hn.ysm.dev",
    );
    let rss = String::from_utf8(feed::rss::render(&view)).unwrap();
    assert!(rss.contains("<title>Story</title>"));
    assert!(!rss.contains("<title>Story ("));
}

#[test]
fn missing_story_time_falls_back_to_plain_title() {
    let mut state = State::empty();
    state
        .stories
        .insert(1, story_with_timing(1, 100, 0, "2025-04-16T03:32:00Z"));
    let view = feed::common::build_view(
        &state,
        100,
        LinkKind::Article,
        FeedFormat::Rss,
        "https://hn.ysm.dev",
    );
    let rss = String::from_utf8(feed::rss::render(&view)).unwrap();
    assert!(rss.contains("<title>Story</title>"));
    assert!(!rss.contains("<title>Story ("));
}

fn unix(iso: &str) -> i64 {
    Timestamp::parse(iso).unwrap().as_datetime().timestamp()
}

fn story_with_timing(id: u64, threshold: u16, story_time: i64, crossed_at: &str) -> Story {
    let crossed = Timestamp::parse(crossed_at).unwrap();
    Story {
        id,
        title: "Story".to_string(),
        url: String::new(),
        hn_url: format!("https://news.ycombinator.com/item?id={id}"),
        score: 120,
        max_score: 120,
        comments: 3,
        by: "alice".to_string(),
        first_seen: crossed.clone(),
        story_time,
        last_output_change_at: crossed.clone(),
        thresholds: BTreeMap::from([(threshold, crossed)]),
    }
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
