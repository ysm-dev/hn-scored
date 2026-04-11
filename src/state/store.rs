use std::{collections::BTreeMap, fs, path::Path};

use serde_json::Value;

use crate::{
    config::STATE_VERSION,
    error::AppError,
    time::Timestamp,
    types::{State, Story},
};

pub fn load_state(path: &Path) -> Result<State, AppError> {
    let bytes = match fs::read(path) {
        Ok(bytes) if bytes.is_empty() => return Ok(State::empty()),
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(State::empty()),
        Err(error) => return Err(AppError::io(path, error)),
    };
    let value: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("[WARN] invalid state.json: {error}");
            return Ok(State::empty());
        }
    };
    parse_state(value)
}

pub fn serialize_state(state: &State) -> Result<Vec<u8>, AppError> {
    let mut bytes = serde_json::to_vec_pretty(state)
        .map_err(|error| AppError::Serialization(error.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn parse_state(value: Value) -> Result<State, AppError> {
    if value.get("version").and_then(Value::as_u64) != Some(STATE_VERSION) {
        eprintln!("[WARN] invalid state.json version");
        return Ok(State::empty());
    }
    let mut stories = BTreeMap::new();
    if let Some(entries) = value.get("stories").and_then(Value::as_object) {
        for (key, story) in entries {
            if let Some((id, story)) = parse_story(key, story) {
                stories.insert(id, story);
            } else {
                eprintln!("[WARN] skipping malformed state entry: {key}");
            }
        }
    }
    let mut state = State {
        version: STATE_VERSION,
        last_output_change_at: Timestamp::epoch(),
        stories,
    };
    state.recompute_last_output_change_at();
    Ok(state)
}

fn parse_story(key: &str, value: &Value) -> Option<(u64, Story)> {
    let id = key.parse().ok()?;
    let object = value.as_object()?;
    let thresholds = object
        .get("thresholds")?
        .as_object()?
        .iter()
        .map(|(k, v)| Some((k.parse().ok()?, Timestamp::parse(v.as_str()?)?)))
        .collect::<Option<BTreeMap<u16, Timestamp>>>()?;
    let story = Story {
        id: object.get("id")?.as_u64()?,
        title: object.get("title")?.as_str()?.to_string(),
        url: object.get("url")?.as_str()?.to_string(),
        hn_url: object.get("hn_url")?.as_str()?.to_string(),
        score: object.get("score")?.as_i64()?,
        max_score: object.get("max_score")?.as_i64()?,
        comments: object.get("comments")?.as_u64()?,
        by: object.get("by")?.as_str()?.to_string(),
        first_seen: Timestamp::parse(object.get("first_seen")?.as_str()?)?,
        story_time: object.get("story_time")?.as_i64()?,
        last_output_change_at: Timestamp::parse(object.get("last_output_change_at")?.as_str()?)?,
        thresholds,
    };
    (story.id == id).then_some((id, story))
}
