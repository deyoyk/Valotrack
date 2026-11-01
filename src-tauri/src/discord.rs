use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use serde_json::Value;

use crate::app::AppState;

pub const DISCORD_APP_ID: &str = "1566350262258499736";

const REFRESH_INTERVAL: Duration = Duration::from_secs(15);
const RECONNECT_DELAY: Duration = Duration::from_secs(5);
const BOARD_TIMEOUT: Duration = Duration::from_secs(8);
const ACTIVITY_NAME: &str = "Valotrack";
const MAX_PARTY_SIZE: i32 = 5;

#[derive(Clone)]
struct Presence {
    details: String,
    state: String,
    party: Option<(i32, i32)>,
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or(0)
}

fn fallback() -> Presence {
    Presence {
        details: "VALORANT companion".to_string(),
        state: "Waiting for game".to_string(),
        party: None,
    }
}

pub fn spawn(state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let started_at = unix_seconds();
        let mut client: Option<DiscordIpcClient> = None;

        loop {
            let settings = state.settings_snapshot();
            if !settings.discord_rpc {
                if let Some(mut active) = client.take() {
                    let _ = active.clear_activity();
                    let _ = active.close();
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }

            if client.is_none() {
                let mut next = DiscordIpcClient::new(DISCORD_APP_ID);
                match next.connect() {
                    Ok(()) => {
                        #[cfg(debug_assertions)]
                        eprintln!("discord: connected");
                        client = Some(next);
                    }
                    Err(_) => {
                        tokio::time::sleep(RECONNECT_DELAY).await;
                        continue;
                    }
                }
            }

            let presence = presence_for(&state).await;
            let mut payload = activity::Activity::new()
                .name(ACTIVITY_NAME)
                .details(presence.details)
                .state(presence.state)
                .timestamps(activity::Timestamps::new().start(started_at));

            if let Some((current, max)) = presence.party {
                payload = payload.party(activity::Party::new().size([current, max]));
            }

            let update_failed = match client.as_mut() {
                Some(active) => active.set_activity(payload).is_err(),
                None => true,
            };

            if update_failed {
                #[cfg(debug_assertions)]
                eprintln!("discord: activity update failed, reconnecting");
                if let Some(mut active) = client.take() {
                    let _ = active.close();
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }

            tokio::time::sleep(REFRESH_INTERVAL).await;
        }
    });
}

async fn presence_for(state: &Arc<AppState>) -> Presence {
    match tokio::time::timeout(BOARD_TIMEOUT, crate::live::board(state)).await {
        Ok(Ok(board)) => presence_from_board(&board),
        _ => fallback(),
    }
}

fn party_of(board: &Value, me: &Value) -> Option<(i32, i32)> {
    let party_id = me
        .get("party")
        .and_then(|party| party.get("id"))
        .and_then(Value::as_str)?;
    let size = board
        .get("parties")
        .and_then(Value::as_array)?
        .iter()
        .find(|party| party.get("id").and_then(Value::as_str) == Some(party_id))
        .and_then(|party| party.get("size"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    (size > 1).then(|| (size.min(MAX_PARTY_SIZE as i64) as i32, MAX_PARTY_SIZE))
}

fn presence_from_board(board: &Value) -> Presence {
    if board.get("available").and_then(Value::as_bool) == Some(false) {
        return fallback();
    }

    let state_label = board
        .get("stateLabel")
        .and_then(Value::as_str)
        .unwrap_or("In lobby");

    if state_label == "Offline" {
        return fallback();
    }

    let mode = board
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("VALORANT");
    let map = board.get("map").and_then(Value::as_str).unwrap_or("Lobby");
    let players = board.get("players").and_then(Value::as_array);

    let Some(me) = players.and_then(|players| {
        players
            .iter()
            .find(|player| player.get("isSelf").and_then(Value::as_bool) == Some(true))
    }) else {
        return Presence {
            details: format!("{map} · {mode}"),
            state: state_label.to_string(),
            party: None,
        };
    };

    let rank = me.get("rank").and_then(Value::as_str).unwrap_or("Unrated");
    let rr = me.get("rr").and_then(Value::as_i64).unwrap_or(0);
    let agent = me
        .get("agent")
        .and_then(Value::as_str)
        .unwrap_or("In lobby");
    let party = party_of(board, me);

    let details = if state_label == "In Game" {
        let ally = board
            .get("score")
            .and_then(|score| score.get("ally"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let enemy = board
            .get("score")
            .and_then(|score| score.get("enemy"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        format!("{map} · {mode} · {ally}:{enemy}")
    } else {
        format!("{map} · {mode}")
    };

    let mut state_text = if rr > 0 {
        format!("{agent} · {rank} · {rr} RR")
    } else {
        format!("{agent} · {rank}")
    };

    if let Some((current, max)) = party {
        state_text = format!("{state_text} · Party {current}/{max}");
    }

    Presence {
        details,
        state: state_text,
        party,
    }
}

#[cfg(test)]
mod tests {
    use super::{presence_from_board, ACTIVITY_NAME};
    use discord_rich_presence::activity;
    use serde_json::json;

    #[test]
    fn offline_board_uses_a_fallback_activity() {
        let presence = presence_from_board(&json!({ "available": false }));

        assert_eq!(presence.details, "VALORANT companion");
        assert_eq!(presence.state, "Waiting for game");
        assert!(presence.party.is_none());
    }

    #[test]
    fn in_game_board_uses_map_mode_score_and_party() {
        let presence = presence_from_board(&json!({
            "stateLabel": "In Game",
            "mode": "Competitive",
            "map": "Ascent",
            "score": { "ally": 7, "enemy": 5 },
            "players": [
                {
                    "isSelf": true,
                    "agent": "Jett",
                    "rank": "Diamond 2",
                    "rr": 44,
                    "party": { "id": "party-1" }
                }
            ],
            "parties": [
                { "id": "party-1", "size": 3 }
            ]
        }));

        assert_eq!(presence.details, "Ascent · Competitive · 7:5");
        assert_eq!(presence.state, "Jett · Diamond 2 · 44 RR · Party 3/5");
        assert_eq!(presence.party, Some((3, 5)));
    }

    #[test]
    fn lobby_without_a_self_player_still_shows_an_activity() {
        let presence = presence_from_board(&json!({
            "stateLabel": "In Lobby",
            "mode": "Swiftplay",
            "map": "Lobby",
            "players": []
        }));

        assert_eq!(presence.details, "Lobby · Swiftplay");
        assert_eq!(presence.state, "In Lobby");
        assert!(presence.party.is_none());
    }

    #[test]
    fn lobby_party_shows_its_size() {
        let presence = presence_from_board(&json!({
            "stateLabel": "In Lobby",
            "mode": "Lobby",
            "map": "Lobby",
            "players": [
                {
                    "isSelf": true,
                    "agent": null,
                    "rank": "Unrated",
                    "rr": 0,
                    "party": { "id": "lobby" }
                }
            ],
            "parties": [
                { "id": "lobby", "size": 2 }
            ]
        }));

        assert_eq!(presence.party, Some((2, 5)));
    }

    #[test]
    fn solo_player_has_no_party() {
        let presence = presence_from_board(&json!({
            "stateLabel": "In Lobby",
            "mode": "Lobby",
            "map": "Lobby",
            "players": [
                {
                    "isSelf": true,
                    "agent": null,
                    "rank": "Unrated",
                    "rr": 0
                }
            ],
            "parties": []
        }));

        assert!(presence.party.is_none());
    }

    #[test]
    fn activity_uses_the_requested_name_and_party_size() {
        let payload = activity::Activity::new()
            .name(ACTIVITY_NAME)
            .details("Ascent · Competitive")
            .state("Jett · Diamond 2")
            .party(activity::Party::new().size([3, 5]));
        let value = serde_json::to_value(payload).expect("activity serializes");

        assert_eq!(value["name"], "Valotrack");
        assert_eq!(value["party"]["size"], json!([3, 5]));
    }
}
