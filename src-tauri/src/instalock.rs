use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use parking_lot::Mutex;
use serde_json::{json, Value};

use crate::{
    app::AppState,
    catalog::{self, Agent},
};

const MIN_DELAY_SECS: f64 = 4.0;
const JITTER_SECS: f64 = 1.5;
const POLL_SECS: f64 = 1.0;
const RECONNECT_SECS: f64 = 1.5;

pub struct InstalockState {
    pub status: Mutex<Value>,
    pub task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

fn jitter_seed() -> f64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or(0);
    (nanos % 1_500_000_000) as f64 / 1_500_000_000.0
}

fn effective_delay(delay: f64, jitter: f64) -> f64 {
    delay.max(MIN_DELAY_SECS) + jitter.clamp(0.0, 1.0) * JITTER_SECS
}

fn normalize_per_map(per_map: &HashMap<String, String>) -> Result<HashMap<String, String>, String> {
    let mut out = HashMap::new();
    for (map, agent) in per_map {
        let map = map.trim().to_lowercase();
        let agent = agent.trim();
        if map.is_empty() || agent.is_empty() {
            continue;
        }
        let Some(resolved) = catalog::resolve_agent(agent) else {
            return Err(format!("Unknown agent '{agent}' for map '{map}'."));
        };
        out.insert(map, resolved.name.to_string());
    }
    Ok(out)
}

/// Reads the agent-select phase from the player's own presence. An unreadable or
/// plaintext presence is treated as unavailable instead of falling back to a guess.
fn self_session_state(presences: &[Value], puuid: &str) -> Option<String> {
    for presence in presences {
        if presence.get("puuid").and_then(Value::as_str) != Some(puuid) {
            continue;
        }
        let product = presence
            .get("product")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !product.is_empty() && product != "valorant" {
            continue;
        }
        let Some(private) = presence.get("private").and_then(Value::as_str) else {
            return None;
        };
        if private.contains('{') {
            return None;
        }
        let Some(data) = crate::riot::decode_private(private) else {
            return None;
        };
        let data = data.get("matchPresenceData").unwrap_or(&data);
        return data
            .get("sessionLoopState")
            .and_then(Value::as_str)
            .map(str::to_string);
    }
    None
}

fn map_id(game: &Value) -> &str {
    game.get("MapID").and_then(Value::as_str).unwrap_or("")
}

fn side_from_match(game: &Value) -> Option<&'static str> {
    match game
        .get("AllyTeam")
        .and_then(|team| team.get("TeamID"))
        .and_then(Value::as_str)
    {
        Some("Red") => Some("Attacker"),
        Some("Blue") => Some("Defender"),
        _ => None,
    }
}

fn pick_for_map(agent: Agent, per_map: &HashMap<String, String>, map_name: &str) -> Agent {
    per_map
        .get(&map_name.to_lowercase())
        .and_then(|name| catalog::resolve_agent(name))
        .unwrap_or(agent)
}

fn finish(
    worker: &InstalockState,
    status: &str,
    message: String,
    agent: Option<&str>,
    map: Option<&str>,
    side: Option<&str>,
) {
    let mut state = worker.status.lock();
    state["running"] = json!(false);
    state["status"] = json!(status);
    state["message"] = json!(message);
    if let Some(agent) = agent {
        state["agent"] = json!(agent);
    }
    if let Some(map) = map {
        state["map"] = json!(map);
    }
    if let Some(side) = side {
        state["side"] = json!(side);
    }
}

impl InstalockState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(json!({
                "running": false,
                "status": "idle",
                "message": "",
                "agent": null,
                "mode": "lock",
                "side": null,
                "map": null
            })),
            task: Mutex::new(None),
        }
    }

    pub fn status(&self) -> Value {
        self.status.lock().clone()
    }

    pub fn stop(&self) {
        if let Some(handle) = self.task.lock().take() {
            handle.abort();
        }
        let mut status = self.status.lock();
        if matches!(
            status.get("status").and_then(Value::as_str),
            Some("waiting") | Some("running")
        ) {
            status["status"] = json!("stopped");
            status["message"] = json!("Stopped.");
        }
        status["running"] = json!(false);
    }

    pub fn start(
        self: &Arc<Self>,
        state: Arc<AppState>,
        agent: String,
        mode: String,
        delay: f64,
        per_map: HashMap<String, String>,
    ) -> Result<Value, String> {
        let agent = catalog::resolve_agent(&agent)
            .ok_or_else(|| format!("Unknown agent '{agent}'"))?;
        let mode = mode.trim().to_lowercase();
        if mode != "lock" && mode != "hover" {
            return Err("Mode must be 'lock' or 'hover'.".to_string());
        }
        if !state.settings_snapshot().allow_live_actions {
            return Err("Live actions are disabled.".to_string());
        }
        let per_map = normalize_per_map(&per_map)?;
        let delay = effective_delay(delay, jitter_seed());
        self.stop();
        let mut status = self.status.lock();
        status["running"] = json!(true);
        status["status"] = json!("waiting");
        status["agent"] = json!(agent.name);
        status["mode"] = json!(mode.clone());
        status["side"] = Value::Null;
        status["map"] = Value::Null;
        status["message"] = json!("Armed — waiting for agent select…");
        drop(status);
        let worker = self.clone();
        let handle = tauri::async_runtime::spawn(loop_pick(
            state,
            worker,
            agent.name,
            mode.clone(),
            delay,
            per_map.clone(),
        ));
        *self.task.lock() = Some(handle);
        Ok(json!({
            "ok": true,
            "running": true,
            "agent": agent.name,
            "mode": mode,
            "status": "waiting",
            "perMap": per_map,
            "delay": (delay * 10.0).round() / 10.0,
            "message": "Armed — waiting for agent select…"
        }))
    }
}

async fn loop_pick(
    state: Arc<AppState>,
    worker: Arc<InstalockState>,
    agent_name: &'static str,
    mode: String,
    delay: f64,
    per_map: HashMap<String, String>,
) {
    let mut done: HashSet<String> = HashSet::new();
    loop {
        let settings = state.settings_snapshot();
        if !settings.allow_live_actions {
            finish(
                &worker,
                "stopped",
                "Live actions were disabled.".to_string(),
                None,
                None,
                None,
            );
            return;
        }
        let session = match state.riot.session(&settings.region).await {
            Ok(session) => session,
            Err(error) => {
                finish(
                    &worker,
                    "error",
                    format!("Couldn't reach the local client: {error}"),
                    None,
                    None,
                    None,
                );
                return;
            }
        };
        let presences = match session.presences().await {
            Ok(presences) => presences,
            Err(_) => {
                finish(
                    &worker,
                    "error",
                    "Local client not reachable — is VALORANT open?".to_string(),
                    None,
                    None,
                    None,
                );
                return;
            }
        };
        let Some(loop_state) = self_session_state(&presences, &session.puuid) else {
            finish(
                &worker,
                "error",
                "Local client not reachable — is VALORANT open?".to_string(),
                None,
                None,
                None,
            );
            return;
        };
        if loop_state == "PREGAME" {
            // Agent select hands out a fresh entitlement; use it for the pick.
            let session = state
                .riot
                .refresh_session(&settings.region)
                .await
                .unwrap_or(session);
            let match_id = session
                .glz_get(&format!("/pregame/v1/players/{}", session.puuid))
                .await
                .ok()
                .and_then(|player| {
                    player
                        .get("MatchID")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .filter(|id| !id.is_empty() && !done.contains(id));
            if let Some(match_id) = match_id {
                if delay > 0.0 {
                    tokio::time::sleep(Duration::from_secs_f64(delay)).await;
                }
                let path = format!("/pregame/v1/matches/{match_id}");
                let mut game = session
                    .glz_get(&path)
                    .await
                    .unwrap_or_else(|_| json!({}));
                let mut tries = 0;
                while !per_map.is_empty() && map_id(&game).is_empty() && tries < 3 {
                    tries += 1;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    if let Ok(fresh) = session.glz_get(&path).await {
                        game = fresh;
                    }
                }
                let side = side_from_match(&game);
                let map_name = catalog::map_name_from_id(map_id(&game));
                let base = catalog::resolve_agent(agent_name).expect("validated agent");
                let choice = pick_for_map(base, &per_map, &map_name);
                let select = session
                    .glz_post(
                        &format!(
                            "/pregame/v1/matches/{match_id}/select/{}",
                            choice.uuid
                        ),
                        None,
                    )
                    .await;
                let Ok(select) = select else {
                    let _ = state.riot.refresh_session(&settings.region).await;
                    tokio::time::sleep(Duration::from_secs_f64(RECONNECT_SECS)).await;
                    continue;
                };
                let mut http_status = select.status().as_u16();
                if mode == "lock" && http_status < 400 {
                    let lock = session
                        .glz_post(
                            &format!(
                                "/pregame/v1/matches/{match_id}/lock/{}",
                                choice.uuid
                            ),
                            None,
                        )
                        .await;
                    let Ok(lock) = lock else {
                        let _ = state.riot.refresh_session(&settings.region).await;
                        tokio::time::sleep(Duration::from_secs_f64(RECONNECT_SECS)).await;
                        continue;
                    };
                    http_status = lock.status().as_u16();
                }
                done.insert(match_id);
                if http_status >= 400 {
                    finish(
                        &worker,
                        "error",
                        format!(
                            "Riot refused {} (HTTP {http_status}) — pick manually.",
                            choice.name
                        ),
                        Some(choice.name),
                        Some(&map_name),
                        side,
                    );
                } else {
                    let action = if mode == "lock" { "Locked" } else { "Hovered" };
                    let mut message = format!("{action} {}", choice.name);
                    if !map_name.is_empty() && map_name != "Unknown" {
                        message.push_str(&format!(" on {map_name}"));
                    }
                    message.push('!');
                    if let Some(side) = side {
                        message.push_str(&format!("  You're {side}."));
                    }
                    finish(
                        &worker,
                        "locked",
                        message,
                        Some(choice.name),
                        Some(&map_name),
                        side,
                    );
                }
                return;
            }
        }
        tokio::time::sleep(Duration::from_secs_f64(POLL_SECS)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};

    fn presence(state: &str) -> Value {
        let private = json!({ "matchPresenceData": { "sessionLoopState": state } }).to_string();
        json!({
            "puuid": "me",
            "product": "valorant",
            "private": STANDARD.encode(private),
        })
    }

    #[test]
    fn delay_keeps_scout_minimum_and_jitter() {
        assert!((effective_delay(0.0, 0.0) - 4.0).abs() < f64::EPSILON);
        assert!((effective_delay(0.0, 1.0) - 5.5).abs() < f64::EPSILON);
        assert!((effective_delay(9.0, 0.5) - 9.75).abs() < f64::EPSILON);
    }

    #[test]
    fn per_map_keys_are_normalized_and_validated() {
        let mut per_map = HashMap::new();
        per_map.insert(" Ascent ".to_string(), "Jett".to_string());
        let normalized = normalize_per_map(&per_map).unwrap();
        assert_eq!(normalized.get("ascent").map(String::as_str), Some("Jett"));

        let mut invalid = HashMap::new();
        invalid.insert("Ascent".to_string(), "NotAnAgent".to_string());
        assert!(normalize_per_map(&invalid).is_err());
    }

    #[test]
    fn per_map_override_wins_for_matching_map_only() {
        let mut per_map = HashMap::new();
        per_map.insert("ascent".to_string(), "Reyna".to_string());
        let jett = catalog::resolve_agent("Jett").unwrap();
        let reyna = catalog::resolve_agent("Reyna").unwrap();
        assert_eq!(
            pick_for_map(catalog::resolve_agent("Jett").unwrap(), &per_map, "Ascent").name,
            reyna.name
        );
        assert_eq!(
            pick_for_map(catalog::resolve_agent("Jett").unwrap(), &per_map, "Bind").name,
            jett.name
        );
    }

    #[test]
    fn session_state_reads_own_presence() {
        assert_eq!(
            self_session_state(&[presence("PREGAME")], "me").as_deref(),
            Some("PREGAME")
        );
        assert_eq!(self_session_state(&[presence("PREGAME")], "someone-else"), None);
    }

    #[test]
    fn session_state_rejects_plaintext_presence() {
        let spoofed = json!({
            "puuid": "me",
            "product": "valorant",
            "private": "{\"sessionLoopState\":\"PREGAME\"}"
        });
        assert_eq!(self_session_state(&[spoofed], "me"), None);
    }

    #[test]
    fn side_comes_from_the_ally_team() {
        assert_eq!(
            side_from_match(&json!({ "AllyTeam": { "TeamID": "Red" } })),
            Some("Attacker")
        );
        assert_eq!(
            side_from_match(&json!({ "AllyTeam": { "TeamID": "Blue" } })),
            Some("Defender")
        );
        assert_eq!(side_from_match(&json!({})), None);
    }

    #[test]
    fn map_id_reads_the_match_payload() {
        assert_eq!(
            map_id(&json!({ "MapID": "/Game/Maps/Ascent/Ascent" })),
            "/Game/Maps/Ascent/Ascent"
        );
        assert_eq!(map_id(&json!({})), "");
    }
}
