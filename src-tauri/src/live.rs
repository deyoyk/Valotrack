use crate::app::AppState;

pub async fn board(state: &AppState) -> Result<serde_json::Value, String> {
    if let Ok(cache) = state.live_cache.lock() {
        if let Some((at, value)) = cache.as_ref() {
            let ttl = state
                .settings
                .read()
                .map(|s| s.live_refresh_secs.max(1))
                .unwrap_or(4);
            if at.elapsed().as_secs() < ttl {
                return Ok(value.clone());
            }
        }
    }

    let status = crate::riot::client_status();
    if !status.available {
        return Ok(serde_json::json!({
            "available": false,
            "state": "OFFLINE",
            "stateLabel": "Offline",
            "source": "local",
            "players": [],
            "teams": {},
            "parties": [],
            "notice": {
                "level": "info",
                "action": "open_game",
                "message": "Open VALORANT to see live ranks, parties and stats."
            }
        }));
    }

    Err("local client session is starting".into())
}
