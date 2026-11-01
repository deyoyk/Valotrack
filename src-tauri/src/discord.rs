use std::sync::Arc;
use std::time::Duration;

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use serde_json::Value;

use crate::app::AppState;

pub fn spawn(state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let mut client: Option<DiscordIpcClient> = None;
        let mut client_id = String::new();
        let mut last_activity: Option<String> = None;
        loop {
            let settings = state.settings_snapshot();
            if !settings.discord_rpc || settings.discord_app_id.trim().is_empty() {
                if let Some(active) = client.as_mut() {
                    let _ = active.clear_activity();
                }
                client = None;
                client_id.clear();
                last_activity = None;
                tokio::time::sleep(Duration::from_secs(15)).await;
                continue;
            }
            let app_id = settings.discord_app_id.trim().to_string();
            if client.is_none() || app_id != client_id {
                let mut next = DiscordIpcClient::new(&app_id);
                if next.connect().is_err() {
                    tokio::time::sleep(Duration::from_secs(20)).await;
                    continue;
                }
                client = Some(next);
                client_id = app_id;
                last_activity = None;
            }
            let payload = activity_for(&state).await;
            let key = payload.as_ref().map(|(details, state_label)| format!("{details}|{state_label}"));
            if key != last_activity {
                if let Some(active) = client.as_mut() {
                    match payload {
                        Some((details, state_label)) => {
                            let activity = activity::Activity::new()
                                .details(&details)
                                .state(&state_label);
                            let _ = active.set_activity(activity);
                        }
                        None => {
                            let _ = active.clear_activity();
                        }
                    }
                }
                last_activity = key;
            }
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    });
}

async fn activity_for(state: &Arc<AppState>) -> Option<(String, String)> {
    let board = crate::live::board(state).await.ok()?;
    if board.get("available").and_then(Value::as_bool) == Some(false) {
        return None;
    }
    let players = board.get("players").and_then(Value::as_array)?;
    let me = players
        .iter()
        .find(|player| player.get("isSelf").and_then(Value::as_bool) == Some(true))?;
    let rank = me.get("rank").and_then(Value::as_str).unwrap_or("Unrated");
    let rr = me.get("rr").and_then(Value::as_i64).unwrap_or(0);
    let agent = me
        .get("agent")
        .and_then(Value::as_str)
        .unwrap_or("In lobby");
    let mode = board
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("VALORANT");
    let map = board.get("map").and_then(Value::as_str).unwrap_or("Lobby");
    let state_label = board
        .get("stateLabel")
        .and_then(Value::as_str)
        .unwrap_or("In lobby");
    let score = board.get("score");
    let details = if state_label == "In Game" {
        let ally = score
            .and_then(|score| score.get("ally"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let enemy = score
            .and_then(|score| score.get("enemy"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        format!("{map} · {mode} · {ally}:{enemy}")
    } else {
        format!("{map} · {mode}")
    };
    let state_text = if rr > 0 {
        format!("{agent} · {rank} · {rr} RR")
    } else {
        format!("{agent} · {rank}")
    };
    Some((details, state_text))
}
