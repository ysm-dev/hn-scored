use crate::{config::RETENTION_DAYS, time::Timestamp, types::State};

pub fn cleanup_state(state: &mut State, cycle_time: &Timestamp) -> bool {
    let cutoff =
        Timestamp::from_datetime(cycle_time.as_datetime() - chrono::Duration::days(RETENTION_DAYS));
    let mut changed = false;
    state.stories.retain(|_, story| {
        let original = story.thresholds.len();
        story
            .thresholds
            .retain(|_, crossed_at| crossed_at.clone() >= cutoff.clone());
        changed |= story.thresholds.len() != original;
        !story.thresholds.is_empty()
    });
    state.recompute_last_output_change_at();
    changed
}
