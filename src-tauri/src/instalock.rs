use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::{json, Value};

use crate::app::AppState;
use crate::catalog;

pub struct InstalockState {
    pub status: Mutex<Value>,
    pub task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
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
        per_map: Value,
    ) -> Result<Value, String> {
        let agent = catalog::resolve_agent(&agent)
            .ok_or_else(|| format!("Unknown agent '{agent}'"))?;
        self.stop();
        let mut status = self.status.lock();
        status["running"] = json!(true);
        status["status"] = json!("waiting");
        status["agent"] = json!(agent.name);
        status["mode"] = json!(mode);
        status["side"] = Value::Null;
        status["map"] = Value::Null;
        status["message"] = json!("Armed - waiting for agent select…");
        drop(status);
        let worker = self.clone();
        let handle = tauri::async_runtime::spawn(loop_pick(
            state,
            worker,
            agent.name,
            mode,
            delay,
            per_map,
        ));
        *self.task.lock() = Some(handle);
        Ok(json!({
            "ok": true,
            "running": true,
            "agent": agent.name,
            "status": "waiting"
        }))
    }
}

async fn loop_pick(
    state: Arc<AppState>,
    worker: Arc<InstalockState>,
    agent_name: &'static str,
    mode: String,
    delay: f64,
    per_map: Value,
) {
    tokio::time::sleep(std::time::Duration::from_secs_f64(delay.max(0.0))).await;
    loop {
        let settings = state.settings_snapshot();
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
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                continue;
            }
        };
        let game_state = session.game_state(&presences);
        if game_state == "PREGAME" {
            if let Ok(Some(current)) = session.current_match("PREGAME").await {
                let map_name = catalog::map_name_from_id(&current.map_id);
                let choice = per_map
                    .get(&map_name.to_lowercase())
                    .and_then(Value::as_str)
                    .and_then(catalog::resolve_agent)
                    .unwrap_or_else(|| catalog::resolve_agent(agent_name).expect("agent"));
                let side = if current
                    .players
                    .iter()
                    .any(|player| player.get("Subject").and_then(Value::as_str) == Some(&session.puuid))
                {
                    current
                        .players
                        .first()
                        .and_then(|p| p.get("TeamID"))
                        .and_then(Value::as_str)
                        .map(|team| match team {
                            "Red" => "Attacker",
                            _ => "Defender",
                        })
                } else {
                    None
                };
                let select = session
                    .glz_post(
                        &format!(
                            "/pregame/v1/matches/{}/select/{}",
                            current.match_id, choice.uuid
                        ),
                        None,
                    )
                    .await;
                let mut ok = select.map(|response| response.status().is_success()).unwrap_or(false);
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
                        ok = response.status().is_success();
                    }
                }
                let mut status = worker.status.lock();
                status["running"] = json!(false);
                status["status"] = json!(if ok { "locked" } else { "error" });
                status["agent"] = json!(choice.name);
                status["side"] = json!(side);
                status["map"] = json!(map_name);
                status["message"] = json!(if ok {
                    format!(
                        "{} {}!",
                        if mode == "lock" { "Locked" } else { "Hovered" },
                        choice.name
                    )
                } else {
                    format!("Riot refused {} - pick manually.", choice.name)
                });
                return;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    }
}
