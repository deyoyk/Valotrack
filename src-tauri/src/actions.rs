use std::sync::Arc;

use serde_json::{json, Value};

use crate::{app::AppState, catalog, riot::Session};

fn preview(message: String) -> Value {
    json!({ "ok": true, "status": "dry-run", "message": message })
}

pub async fn queue_action(
    state: &Arc<AppState>,
    action: &str,
    queue_id: Option<String>,
    dry_run: bool,
) -> Value {
    let settings = state.settings_snapshot();
    if dry_run || !settings.allow_live_actions {
        let label = queue_id
            .as_deref()
            .map(catalog::mode_label)
            .unwrap_or("the selected mode");
        let message = match action {
            "select" => format!("DRY-RUN: would switch to {label}."),
            "start" => "DRY-RUN: would start the queue.".to_string(),
            "stop" => "DRY-RUN: would cancel the queue.".to_string(),
            _ => "DRY-RUN: no action taken.".to_string(),
        };
        return preview(message);
    }
    let session = match state.riot.session(&settings.region).await {
        Ok(session) => session,
        Err(error) => return json!({ "ok": false, "message": error.to_string() }),
    };
    let snapshot = match session.party_snapshot().await {
        Ok(snapshot) => snapshot,
        Err(error) => return json!({ "ok": false, "message": error.to_string() }),
    };
    if snapshot.get("available").and_then(Value::as_bool) != Some(true) {
        return json!({
            "ok": false,
            "message": "Not in a party - get to the VALORANT lobby first."
        });
    }
    let party_id = snapshot
        .get("partyId")
        .and_then(Value::as_str)
        .unwrap_or("");
    match action {
        "select" => {
            let Some(queue_id) = queue_id else {
                return json!({ "ok": false, "message": "Pick a game mode first." });
            };
            let eligible = snapshot
                .get("eligible")
                .and_then(Value::as_array)
                .map(|queues| {
                    queues
                        .iter()
                        .filter_map(|queue| queue.get("id").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if !eligible.is_empty() && !eligible.contains(&queue_id.as_str()) {
                return json!({
                    "ok": false,
                    "message": format!("{} isn't available right now.", catalog::mode_label(&queue_id))
                });
            }
            match session
                .glz_post(
                    &format!("/parties/v1/parties/{party_id}/queue"),
                    Some(&json!({ "queueID": queue_id })),
                )
                .await
            {
                Ok(response) if response.status().is_success() => json!({
                    "ok": true,
                    "status": "selected",
                    "message": format!("Game mode set to {}.", catalog::mode_label(&queue_id)),
                    "queue": session.party_snapshot().await.unwrap_or(Value::Null)
                }),
                Ok(response) => json!
                    ({ "ok": false, "message": format!("Riot refused the change (HTTP {}).", response.status()) }),
                Err(error) => json!({ "ok": false, "message": error.to_string() }),
            }
        }
        "start" => {
            if snapshot.get("inQueue").and_then(Value::as_bool) == Some(true) {
                return json!({ "ok": true, "status": "queued", "inQueue": true, "message": "Already in queue." });
            }
            if snapshot.get("isOwner").and_then(Value::as_bool) == Some(false) {
                return json!({ "ok": false, "message": "Only the party owner can start the queue." });
            }
            if snapshot.get("allReady").and_then(Value::as_bool) == Some(false) {
                return json!({ "ok": false, "message": "Not everyone in the party is ready." });
            }
            match session
                .glz_post(
                    &format!("/parties/v1/parties/{party_id}/matchmaking/join"),
                    None,
                )
                .await
            {
                Ok(response) if response.status().is_success() => json!({
                    "ok": true,
                    "status": "queued",
                    "inQueue": true,
                    "message": "Queue started.",
                    "queue": session.party_snapshot().await.unwrap_or(Value::Null)
                }),
                Ok(response) => json!({ "ok": false, "message": format!("Riot refused the queue (HTTP {}).", response.status()) }),
                Err(error) => json!({ "ok": false, "message": error.to_string() }),
            }
        }
        "stop" => match session
            .glz_post(
                &format!("/parties/v1/parties/{party_id}/matchmaking/leave"),
                None,
            )
            .await
        {
            Ok(response) if response.status().is_success() => json!({
                "ok": true,
                "status": "idle",
                "inQueue": false,
                "message": "Queue cancelled.",
                "queue": session.party_snapshot().await.unwrap_or(Value::Null)
            }),
            Ok(response) => json!({ "ok": false, "message": format!("Riot refused the cancel (HTTP {}).", response.status()) }),
            Err(error) => json!({ "ok": false, "message": error.to_string() }),
        },
        _ => json!({ "ok": false, "message": "Unknown queue action." }),
    }
}

pub async fn dodge(state: &Arc<AppState>, dry_run: bool) -> Value {
    let settings = state.settings_snapshot();
    if dry_run || !settings.allow_live_actions {
        return preview("DRY-RUN: would dodge agent select.".into());
    }
    let session = match state.riot.session(&settings.region).await {
        Ok(session) => session,
        Err(error) => return json!({ "ok": false, "message": error.to_string() }),
    };
    let presences = session.presences().await.unwrap_or_default();
    let phase = session.game_state(&presences);
    let player = match session
        .glz_get(&format!("/pregame/v1/players/{}", session.puuid))
        .await
    {
        Ok(player) => player,
        Err(error) => {
            return json!({
                "ok": false,
                "status": "lookup-failed",
                "phase": phase,
                "message": format!("Could not read agent select: {error}")
            })
        }
    };
    let Some(match_id) = player
        .get("MatchID")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
    else {
        return json!({
            "ok": false,
            "status": "not-in-pregame",
            "phase": phase,
            "message": if phase == "PREGAME" {
                "Agent select is active, but Riot did not return a match ID. Try again in a moment.".to_string()
            } else {
                format!("Not in agent select (current phase: {phase}).")
            }
        });
    };
    let path = format!("/pregame/v1/matches/{match_id}/quit");
    let first = quit_pregame(&session, &path, Some(&json!({}))).await;
    let attempt = match first {
        Ok((status, detail)) if (200..=299).contains(&status) => Ok((status, detail)),
        Ok((status, detail)) if (400..=422).contains(&status) => {
            match quit_pregame(&session, &path, None).await {
                Ok(retry) => Ok(retry),
                Err(_) => Ok((status, detail)),
            }
        }
        other => other,
    };
    match attempt {
        Ok((status, _)) if (200..300).contains(&status) => json!({
            "ok": true,
            "status": "dodged",
            "phase": phase,
            "matchId": match_id,
            "message": "Dodged agent select."
        }),
        Ok((status, detail)) => json!({
            "ok": false,
            "status": "refused",
            "phase": phase,
            "matchId": match_id,
            "httpStatus": status,
            "detail": detail,
            "message": format!("Riot refused the dodge (HTTP {status}).")
        }),
        Err(error) => json!({
            "ok": false,
            "status": "request-failed",
            "phase": phase,
            "matchId": match_id,
            "message": format!("Dodge request failed: {error}")
        }),
    }
}

async fn quit_pregame(
    session: &Session,
    path: &str,
    body: Option<&Value>,
) -> Result<(u16, String), String> {
    let response = session
        .glz_post(path, body)
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let text = response.text().await.unwrap_or_default();
    Ok((status, text.chars().take(240).collect()))
}

pub async fn check_side(state: &Arc<AppState>) -> Value {
    let settings = state.settings_snapshot();
    let session = match state.riot.session(&settings.region).await {
        Ok(session) => session,
        Err(error) => return json!({ "ok": false, "message": error.to_string() }),
    };
    match session.current_match("PREGAME").await {
        Ok(Some(current)) => {
            let team = current
                .players
                .iter()
                .find(|player| {
                    player.get("Subject").and_then(Value::as_str) == Some(session.puuid.as_str())
                })
                .and_then(|player| player.get("TeamID"))
                .and_then(Value::as_str)
                .unwrap_or("Blue");
            json!({
                "ok": true,
                "side": if team == "Red" { "Attacker" } else { "Defender" },
                "map": catalog::map_name_from_id(&current.map_id),
                "message": if team == "Red" {
                    "You start on attack."
                } else {
                    "You start on defense."
                }
            })
        }
        Ok(None) => json!({ "ok": false, "message": "No agent-select match found." }),
        Err(error) => json!({ "ok": false, "message": error.to_string() }),
    }
}

async fn pregame_match_id(session: &Session) -> Option<String> {
    session
        .glz_get(&format!("/pregame/v1/players/{}", session.puuid))
        .await
        .ok()
        .and_then(|value| value.get("MatchID").and_then(Value::as_str).map(str::to_string))
}

pub async fn live_instalock(
    state: &Arc<AppState>,
    agent: &str,
    mode: &str,
    dry_run: bool,
) -> Value {
    let Some(agent) = catalog::resolve_agent(agent) else {
        return json!({ "ok": false, "message": "Unknown agent." });
    };
    let settings = state.settings_snapshot();
    let session = match state.riot.session(&settings.region).await {
        Ok(session) => session,
        Err(error) => return json!({ "ok": false, "message": error.to_string() }),
    };
    let owned = match crate::content::owned_ids_result(&session, crate::content::ITEM_AGENT).await {
        Ok(owned) => owned,
        Err(error) => {
            return json!({
                "ok": false,
                "message": format!("Could not verify agent ownership: {error}")
            })
        }
    };
    if !crate::catalog::is_agent_owned(&owned, agent.uuid) {
        return json!({
            "ok": false,
            "message": format!("You don't own {} on this account.", agent.name)
        });
    }
    if dry_run || !settings.allow_live_actions {
        return json!({
            "ok": true,
            "status": "dry-run",
            "agent": agent.name,
            "message": format!("DRY-RUN: would {mode} {} when agent select opens.", agent.name)
        });
    }
    let Some(match_id) = pregame_match_id(&session).await else {
        return json!({ "ok": false, "message": "Not in agent select." });
    };
    let mut ok = session
        .glz_post(
            &format!("/pregame/v1/matches/{match_id}/select/{}", agent.uuid),
            None,
        )
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false);
    if ok && mode == "lock" {
        ok = session
            .glz_post(
                &format!("/pregame/v1/matches/{match_id}/lock/{}", agent.uuid),
                None,
            )
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false);
    }
    json!({
        "ok": ok,
        "status": if ok { "locked" } else { "error" },
        "agent": agent.name,
        "message": if ok {
            format!("{mode}ed {}.", agent.name)
        } else {
            format!("Riot refused {}.", agent.name)
        }
    })
}
