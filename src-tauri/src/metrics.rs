use std::{path::Path, sync::Arc, time::Duration};

use serde_json::{json, Value};
use uuid::{Uuid, Variant};

use crate::app::AppState;

const METRICS_URL: &str = "https://metrics.valotracker.rymelabs.dev/v1/metrics";
const PRESENCE_URL: &str = "https://metrics.valotracker.rymelabs.dev/v1/presence";
const CACHE_KEY: &str = "metrics:active-users";
const INSTALLATION_FILE: &str = "installation-id";
const CACHE_TTL_SECS: u64 = 300;
const HEARTBEAT_SECS: u64 = 240;

fn valid_installation_id(value: &str) -> bool {
    Uuid::parse_str(value)
        .map(|id| id.get_version_num() == 4 && id.get_variant() == Variant::RFC4122)
        .unwrap_or(false)
}

fn installation_id(data_dir: &Path) -> Result<String, String> {
    let path = data_dir.join(INSTALLATION_FILE);
    if let Ok(raw) = std::fs::read_to_string(&path) {
        let existing = raw.trim();
        if valid_installation_id(existing) {
            return Ok(existing.to_string());
        }
    }

    let created = Uuid::new_v4().to_string();
    std::fs::create_dir_all(data_dir).map_err(|error| error.to_string())?;
    std::fs::write(&path, &created).map_err(|error| error.to_string())?;
    Ok(created)
}

fn read_active_users(payload: &Value) -> Value {
    json!({
        "activeUsers": payload.get("activeUsers").and_then(Value::as_i64).unwrap_or(0),
        "updatedAt": payload.get("updatedAt").cloned().unwrap_or(Value::Null)
    })
}

pub async fn snapshot(state: &Arc<AppState>) -> Result<Value, String> {
    if let Some(cached) = state.cache_json_for(CACHE_KEY, Duration::from_secs(CACHE_TTL_SECS)) {
        return Ok(cached);
    }
    let response = state
        .http
        .get(METRICS_URL)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|error| format!("Metrics unavailable: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Metrics returned HTTP {}.", response.status()));
    }
    let payload: Value = response
        .json()
        .await
        .map_err(|error| format!("Metrics response could not be read: {error}"))?;
    let value = read_active_users(&payload);
    state.put_json(CACHE_KEY.to_string(), value.clone());
    Ok(value)
}

pub async fn heartbeat(state: &Arc<AppState>) -> Result<Value, String> {
    let installation = installation_id(&state.data_dir)?;
    let response = state
        .http
        .post(PRESENCE_URL)
        .json(&json!({ "installationId": installation }))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|error| format!("Presence unavailable: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Presence returned HTTP {}.", response.status()));
    }
    let payload: Value = response
        .json()
        .await
        .map_err(|error| format!("Presence response could not be read: {error}"))?;
    let value = read_active_users(&payload);
    state.put_json(CACHE_KEY.to_string(), value.clone());
    Ok(value)
}

/**
 * Keeps this installation marked active while the app is running. The server
 * hashes the anonymous installation ID and returns the current count, so the
 * sidebar does not need a second metrics request most of the time.
 */
pub fn spawn(state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        loop {
            let _ = heartbeat(&state).await;
            tokio::time::sleep(Duration::from_secs(HEARTBEAT_SECS)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::valid_installation_id;

    #[test]
    fn accepts_generated_installation_ids() {
        assert!(valid_installation_id("9f1c2b3a-4d5e-4f60-8a7b-1c2d3e4f5a6b"));
    }

    #[test]
    fn rejects_other_identifiers() {
        assert!(!valid_installation_id("user-1"));
        assert!(!valid_installation_id("9f1c2b3a-4d5e-1f60-8a7b-1c2d3e4f5a6b"));
        assert!(!valid_installation_id("9f1c2b3a-4d5e-4f60-0a7b-1c2d3e4f5a6b"));
    }
}
