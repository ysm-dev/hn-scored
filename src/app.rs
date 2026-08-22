use std::{collections::BTreeSet, time::Instant};

use crate::{
    api::firebase::FirebaseClient,
    config::THRESHOLDS,
    error::AppError,
    feed,
    output::{self, Artifacts},
    state,
    time::Timestamp,
    types::{FeedFormat, LinkKind},
};

pub async fn run_once(
    config: &crate::config::AppConfig,
    cycle_time: Timestamp,
    client: Option<FirebaseClient>,
) -> Result<u8, AppError> {
    let started = Instant::now();
    let client = client.unwrap_or_else(|| FirebaseClient::new(&config.api_base_url));
    let mut state = state::store::load_state(&config.state_path)?;
    state::cleanup::cleanup_state(&mut state, &cycle_time);
    let discovery = client.fetch_discovery().await;
    let successful_lists: Vec<_> = discovery
        .iter()
        .filter_map(|entry| entry.ids.as_ref().ok())
        .collect();
    if successful_lists.is_empty() {
        println!(
            "[{cycle_time}] fetched=0 new=0 crossings=0 dead=0 changed=false duration={:.1}s",
            started.elapsed().as_secs_f32()
        );
        return Ok(1);
    }
    let mut fetch_set = BTreeSet::new();
    for ids in successful_lists {
        fetch_set.extend(ids.iter().copied());
    }
    fetch_set.extend(state.retained_ids());
    let mut results = client.fetch_items(fetch_set.into_iter().collect()).await;
    results.sort_by_key(|result| result.id);
    let mut fetched = 0;
    let mut created = 0;
    let mut dead = 0;
    let mut crossings = 0;
    for result in results {
        let Ok(item) = result.item else { continue };
        fetched += 1;
        let current = state.stories.get(&item.id).cloned();
        let previous_max_score = state.max_scores.get(&item.id).copied();
        let (next, stats) =
            state::update::apply_item(current.as_ref(), previous_max_score, &item, &cycle_time);
        created += usize::from(stats.created);
        dead += usize::from(stats.dead_removed);
        crossings += stats.crossings;
        if let Some(score) = stats.observed_score {
            state
                .max_scores
                .entry(item.id)
                .and_modify(|maximum| *maximum = (*maximum).max(score))
                .or_insert(score);
        }
        match next {
            Some(story) => {
                state.stories.insert(story.id, story);
            }
            None => {
                state.stories.remove(&item.id);
            }
        }
    }
    state.recompute_last_output_change_at();
    let artifacts = build_artifacts(&state, &config.base_url)?;
    let changed =
        output::has_persisted_changes(&config.state_path, &config.output_dir, &artifacts)?;
    if changed {
        output::persist(&config.state_path, &config.output_dir, &artifacts)?;
    }
    println!(
        "[{cycle_time}] fetched={fetched} new={created} crossings={crossings} dead={dead} changed={changed} duration={:.1}s",
        started.elapsed().as_secs_f32()
    );
    Ok(if changed { 0 } else { 2 })
}

fn build_artifacts(state: &crate::types::State, base_url: &str) -> Result<Artifacts, AppError> {
    let mut artifacts = Artifacts {
        headers_bytes: feed::common::headers_content().into_bytes(),
        index_bytes: crate::html::index::render(base_url, &state.last_output_change_at),
        state_bytes: state::store::serialize_state(state)?,
        ..Artifacts::default()
    };
    for threshold in THRESHOLDS {
        for kind in [LinkKind::Article, LinkKind::Comments] {
            for format in [FeedFormat::Rss, FeedFormat::Atom, FeedFormat::Json] {
                let view = feed::common::build_view(state, threshold, kind, format, base_url);
                let bytes = match format {
                    FeedFormat::Rss => feed::rss::render(&view),
                    FeedFormat::Atom => feed::atom::render(&view),
                    FeedFormat::Json => feed::json_feed::render(&view),
                };
                artifacts
                    .feed_files
                    .insert(feed::common::feed_path(kind, format, threshold), bytes);
            }
        }
    }
    Ok(artifacts)
}
