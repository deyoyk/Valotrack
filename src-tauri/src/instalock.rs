use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use parking_lot::Mutex;
use serde_json::{json, Value};

use crate::{
    app::AppState,
    catalog::{self, Agent},
    riot::CurrentMatch,
};

const MIN_DELAY_SECS: f64 = 4.0;
const JITTER_SECS: f64 = 1.5;

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

fn team_side(current: &CurrentMatch, puuid: &str) -> Option<&'static str> {
    let team = current
        .players
        .iter()
        .find(|player| player.get("Subject").and_then(Value::as_str) == Some(puuid))
        .and_then(|player| player.get("TeamID"))
        .and_then(Value::as_str)
        .or_else(|| {
            current
                .players
                .first()
                .and_then(|player| player.get("TeamID"))
                .and_then(Value::as_str)
        })?;
    Some(if team == "Red" {
        "Attacker"
    } else {
        "Defender"
    })
}

fn pick_for_map(agent: Agent, per_map: &HashMap<String, String>, map_name: &str) -> Agent {
    per_map
        .get(&map_name.to_lowercase())
        .and_then(|name| catalog::resolve_agent(name))
        .unwrap_or(agent)
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
        status["message"] = json!("Armed - waiting for agent select...");
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
            "message": "Armed - waiting for agent select..."
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
    let mut offline_since: Option<Instant> = None;
    loop {
        let settings = state.settings_snapshot();
        if !settings.allow_live_actions {
            let mut status = worker.status.lock();
            status["running"] = json!(false);
            status["status"] = json!("stopped");
            status["message"] = json!("Live actions were disabled.");
            return;
        }
        let session = match state.riot.session(&settings.region).await {
            Ok(session) => session,
            Err(error) => {
                let mut status = worker.status.lock();
                status["running"] = json!(false);
                status["status"] = json!("error");
                status["message"] = json!(error.to_string());
                return;
            }
        };
        let presences = match session.presences().await {
            Ok(presences) => presences,
            Err(_) => {
                let since = offline_since.get_or_insert_with(Instant::now);
                if since.elapsed() > Duration::from_secs(15) {
                    let mut status = worker.status.lock();
                    status["running"] = json!(false);
                    status["status"] = json!("error");
                    status["message"] =
                        json!("Local client not reachable - is VALORANT open and signed in?");
                    return;
                }
                tokio::time::sleep(Duration::from_millis(1500)).await;
                continue;
            }
        };
        let game_state = session.game_state(&presences);
        if game_state == "OFFLINE" {
            let since = offline_since.get_or_insert_with(Instant::now);
            if since.elapsed() > Duration::from_secs(15) {
                let mut status = worker.status.lock();
                status["running"] = json!(false);
                status["status"] = json!("error");
                status["message"] =
                    json!("Local client not reachable - is VALORANT open and signed in?");
                return;
            }
        } else {
            offline_since = None;
        }
        if game_state == "PREGAME" {
            let Some(mut current) = session
                .current_match("PREGAME")
                .await
                .ok()
                .flatten()
            else {
                tokio::time::sleep(Duration::from_millis(1200)).await;
                continue;
            };
            let mut map_name = catalog::map_name_from_id(&current.map_id);
            let mut tries = 0;
            while (map_name.is_empty() || map_name == "Unknown") && tries < 3 {
                tries += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
                if let Ok(Some(fresh)) = session.current_match("PREGAME").await {
                    current = fresh;
                    map_name = catalog::map_name_from_id(&current.map_id);
                }
            }
            tokio::time::sleep(Duration::from_secs_f64(delay)).await;
            if !state.settings_snapshot().allow_live_actions {
                let mut status = worker.status.lock();
                status["running"] = json!(false);
                status["status"] = json!("stopped");
                status["message"] = json!("Live actions were disabled.");
                return;
            }
            let base = catalog::resolve_agent(agent_name).expect("validated agent");
            let choice = pick_for_map(base, &per_map, &map_name);
            let side = team_side(&current, &session.puuid);
            let select = session
                .glz_post(
                    &format!(
                        "/pregame/v1/matches/{}/select/{}",
                        current.match_id, choice.uuid
                    ),
                    None,
                )
                .await;
            let select_status = select.map(|response| response.status().as_u16()).unwrap_or(0);
            let mut ok = (200..300).contains(&select_status);
            let mut http_status = select_status;
            if ok && mode == "lock" {
                if let Ok(response) = session
                    .glz_post(
                        &format!(
                            "/pregame/v1/matches/{}/lock/{}",
                            current.match_id, choice.uuid
                        ),
                        None,
                    )
                    .await
                {
                    http_status = response.status().as_u16();
                    ok = (200..300).contains(&http_status);
                } else {
                    ok = false;
                    http_status = 0;
                }
            }
            let mut status = worker.status.lock();
            status["running"] = json!(false);
            status["status"] = json!(if ok { "locked" } else { "error" });
            status["agent"] = json!(choice.name);
            status["side"] = json!(side);
            status["map"] = json!(map_name);
            let action = if mode == "lock" { "Locked" } else { "Hovered" };
            status["message"] = json!(if ok {
                format!(
                    "{action} {}{}{}",
                    choice.name,
                    if map_name.is_empty() || map_name == "Unknown" {
                        String::new()
                    } else {
                        format!(" on {map_name}")
                    },
                    side.map(|side| format!(" - you're {side}."))
                        .unwrap_or_else(|| ".".to_string())
                )
            } else if http_status >= 400 {
                format!(
                    "Riot refused {} (HTTP {http_status}) - pick manually.",
                    choice.name
                )
            } else {
                format!(
                    "Couldn't reach agent select for {} - pick manually.",
                    choice.name
                )
            });
            return;
        }
        tokio::time::sleep(Duration::from_millis(1200)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn side_comes_from_the_self_player() {
        let current = CurrentMatch {
            players: vec![json!({ "Subject": "mate", "TeamID": "Blue" })],
            match_id: "match".to_string(),
            map_id: String::new(),
            queue: String::new(),
        };
        assert_eq!(team_side(&current, "mate"), Some("Defender"));

        let current = CurrentMatch {
            players: vec![json!({ "Subject": "mate", "TeamID": "Red" })],
            match_id: "match".to_string(),
            map_id: String::new(),
            queue: String::new(),
        };
        assert_eq!(team_side(&current, "mate"), Some("Attacker"));
    }
}
