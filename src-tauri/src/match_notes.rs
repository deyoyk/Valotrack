use std::sync::Arc;

use serde_json::{json, Value};

use crate::{
    app::{store, AppState},
    history,
};

pub fn get(state: &Arc<AppState>, match_id: &str) -> Value {
    let data = store::read(&state.data_dir, "match_notes");
    data.get("matches")
        .and_then(Value::as_object)
        .and_then(|matches| matches.get(match_id))
        .cloned()
        .unwrap_or_else(|| json!({ "matchId": match_id, "note": "", "tags": [] }))
}

pub fn update(
    state: &Arc<AppState>,
    match_id: &str,
    note: &str,
    tags: Vec<String>,
) -> Result<Value, String> {
    let mut data = store::read(&state.data_dir, "match_notes");
    if data.get("matches").is_none() {
        data["matches"] = json!({});
    }
    let clean_tags: Vec<String> = tags
        .into_iter()
        .map(|tag| tag.trim().to_lowercase())
        .filter(|tag| !tag.is_empty())
        .take(8)
        .collect();
    let entry = json!({
        "matchId": match_id,
        "note": note.trim().chars().take(2000).collect::<String>(),
        "tags": clean_tags,
        "updatedAt": history::now()
    });
    data["matches"][match_id] = entry.clone();
    store::write(&state.data_dir, "match_notes", &data)?;
    Ok(entry)
}

pub fn all(state: &Arc<AppState>) -> Value {
    let data = store::read(&state.data_dir, "match_notes");
    data.get("matches").cloned().unwrap_or_else(|| json!({}))
}
