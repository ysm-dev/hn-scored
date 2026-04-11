use std::collections::BTreeMap;

use crate::{config::THRESHOLDS, time::Timestamp};

pub fn record_crossings(
    thresholds: &mut BTreeMap<u16, Timestamp>,
    score: i64,
    cycle_time: &Timestamp,
) -> usize {
    let mut added = 0;
    for threshold in crossed_thresholds(score) {
        if let std::collections::btree_map::Entry::Vacant(entry) = thresholds.entry(threshold) {
            entry.insert(cycle_time.clone());
            added += 1;
        }
    }
    added
}

pub fn crossed_thresholds(score: i64) -> Vec<u16> {
    THRESHOLDS
        .iter()
        .copied()
        .filter(|threshold| *threshold == 0 || score >= i64::from(*threshold))
        .collect()
}
