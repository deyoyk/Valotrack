use std::{collections::HashMap, sync::Arc};

use base64::Engine;
use serde_json::{json, Value};

use crate::{app::AppState, catalog, riot::Session};

fn private_presence(presence: &Value) -> Value {
    let Some(raw) = presence.get("private").and_then(Value::as_str) else {
        return json!({});
    };
    if raw.is_empty() {
        return json!({});
    }
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(raw) {
        if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
            return value;
        }
    }
    serde_json::from_str(raw).unwrap_or_else(|_| json!({}))
}

fn presence_summary(presence: &Value) -> Value {
    let private = private_presence(presence);
    let raw_state = presence.get("state").and_then(Value::as_str).unwrap_or("offline");
    let raw_product = presence.get("product").and_then(Value::as_str).unwrap_or("");
    let stale_mobile = raw_state == "mobile"
        && !raw_product.is_empty()
        && raw_product != "valorant"
        && private.as_object().map(|value| value.is_empty()).unwrap_or(true);
    let state = if stale_mobile { "offline" } else { raw_state };
    let product = if stale_mobile { "" } else { raw_product };
    let party = private.get("partyPresenceData").unwrap_or(&private);
    let match_data = private.get("matchPresenceData").unwrap_or(&private);
    let map_id = match_data
        .get("matchMap")
        .and_then(Value::as_str)
        .unwrap_or("");
    let map = if map_id.is_empty() {
        Value::Null
    } else {
        json!(catalog::map_name_from_id(map_id))
    };
    let queue_id = match_data
        .get("queueId")
        .and_then(Value::as_str)
        .or_else(|| private.get("queueId").and_then(Value::as_str))
        .unwrap_or("");
    json!({
        "state": state,
        "product": product,
        "platform": presence.get("platform").and_then(Value::as_str).unwrap_or(""),
        "map": map,
        "queue": if queue_id.is_empty() { Value::Null } else { json!(catalog::mode_label(queue_id)) },
        "partySize": party.get("partySize").and_then(Value::as_i64).unwrap_or(1),
        "partyId": party.get("partyId").cloned().unwrap_or(Value::Null)
    })
}

async fn account(session: &Session) -> String {
    session
        .local_get("/player-account/aliases/v1/active")
        .await
        .ok()
        .and_then(|value| {
            let name = value.get("game_name").and_then(Value::as_str).unwrap_or("");
            let tag = value.get("tag_line").and_then(Value::as_str).unwrap_or("");
            (!name.is_empty()).then(|| {
                if tag.is_empty() {
                    name.to_string()
                } else {
                    format!("{name}#{tag}")
                }
            })
        })
        .unwrap_or_else(|| session.puuid.clone())
}

pub async fn snapshot(state: &Arc<AppState>) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let friends_data = session
        .local_get("/chat/v4/friends")
        .await
        .map_err(|error| format!("Friends unavailable: {error}"))?;
    let presences_data = session.local_get("/chat/v4/presences").await.unwrap_or_else(|_| json!({}));
    let requests_data = session
        .local_get("/chat/v4/friendrequests")
        .await
        .unwrap_or_else(|_| json!({}));
    let conversations_data = session
        .local_get("/chat/v6/conversations")
        .await
        .unwrap_or_else(|_| json!({}));
    let participants_data = session
        .local_get("/chat/v5/participants")
        .await
        .unwrap_or_else(|_| json!({}));

    let by_puuid: HashMap<String, Value> = presences_data
        .get("presences")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|presence| {
                    let puuid = presence.get("puuid").and_then(Value::as_str)?;
                    Some((puuid.to_string(), presence.clone()))
                })
                .collect()
        })
        .unwrap_or_default();
    let friends = friends_data
        .get("friends")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|friend| {
                    let puuid = friend.get("puuid").and_then(Value::as_str).unwrap_or("");
                    let presence = by_puuid.get(puuid);
                    let summary = presence.map(presence_summary).unwrap_or_else(|| {
                        json!({
                            "state": "offline",
                            "product": "",
                            "platform": "",
                            "map": Value::Null,
                            "queue": Value::Null,
                            "partySize": 0,
                            "partyId": Value::Null
                        })
                    });
                    json!({
                        "puuid": puuid,
                        "pid": friend.get("pid").cloned().unwrap_or(Value::Null),
                        "gameName": friend.get("game_name").cloned().unwrap_or(Value::Null),
                        "tagLine": friend.get("game_tag").cloned().unwrap_or(Value::Null),
                        "group": friend.get("group").cloned().unwrap_or(Value::Null),
                        "note": friend.get("note").cloned().unwrap_or(Value::Null),
                        "online": presence.is_some() && summary.get("state").and_then(Value::as_str) != Some("offline"),
                        "lastOnline": friend.get("last_online_ts").cloned().unwrap_or(Value::Null),
                        "presence": summary
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let requests = requests_data
        .get("requests")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|request| {
                    json!({
                        "puuid": request.get("puuid").cloned().unwrap_or(Value::Null),
                        "pid": request.get("pid").cloned().unwrap_or(Value::Null),
                        "gameName": request.get("game_name").cloned().unwrap_or(Value::Null),
                        "tagLine": request.get("game_tag").cloned().unwrap_or(Value::Null),
                        "subscription": request.get("subscription").cloned().unwrap_or(Value::Null),
                        "note": request.get("note").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let participants_by_cid: HashMap<String, Vec<Value>> = participants_data
        .get("participants")
        .and_then(Value::as_array)
        .map(|rows| {
            let mut map: HashMap<String, Vec<Value>> = HashMap::new();
            for row in rows {
                let Some(cid) = row.get("cid").and_then(Value::as_str) else {
                    continue;
                };
                map.entry(cid.to_string()).or_default().push(json!({
                    "puuid": row.get("puuid").cloned().unwrap_or(Value::Null),
                    "gameName": row.get("game_name").cloned().unwrap_or(Value::Null),
                    "tagLine": row.get("game_tag").cloned().unwrap_or(Value::Null)
                }));
            }
            map
        })
        .unwrap_or_default();
    let conversations = conversations_data
        .get("conversations")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|conversation| {
                    let cid = conversation.get("cid").and_then(Value::as_str).unwrap_or("");
                    json!({
                        "cid": cid,
                        "type": conversation.get("type").cloned().unwrap_or(Value::Null),
                        "unread": conversation.get("unread_count").and_then(Value::as_i64).unwrap_or(0),
                        "muted": conversation.get("muted").and_then(Value::as_bool).unwrap_or(false),
                        "directMessages": conversation.get("direct_messages").and_then(Value::as_bool).unwrap_or(false),
                        "participants": participants_by_cid.get(cid).cloned().unwrap_or_default()
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(json!({
        "available": true,
        "puuid": session.puuid,
        "account": account(&session).await,
        "friends": friends,
        "requests": requests,
        "conversations": conversations
    }))
}

pub async fn messages(state: &Arc<AppState>, cid: &str) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let encoded = urlencoding::encode(cid);
    session
        .local_get(&format!("/chat/v6/messages?cid={encoded}"))
        .await
        .map_err(|error| format!("Chat history unavailable: {error}"))
}

pub async fn send(
    state: &Arc<AppState>,
    cid: &str,
    message: &str,
    kind: &str,
) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    session
        .local_post(
            "/chat/v6/messages",
            Some(&json!({
                "cid": cid,
                "message": message,
                "type": if kind == "groupchat" { "groupchat" } else { "chat" }
            })),
        )
        .await
        .map_err(|error| format!("Message failed: {error}"))
}

pub async fn send_request(
    state: &Arc<AppState>,
    name: &str,
    tag: &str,
) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    session
        .local_post(
            "/chat/v4/friendrequests",
            Some(&json!({ "game_name": name, "game_tag": tag })),
        )
        .await
        .map_err(|error| format!("Friend request failed: {error}"))
}

pub async fn remove_request(state: &Arc<AppState>, puuid: &str) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    session
        .local_delete(
            "/chat/v4/friendrequests",
            Some(&json!({ "puuid": puuid })),
        )
        .await
        .map_err(|error| format!("Couldn't remove the request: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_mobile_league_presence_is_offline() {
        let summary = presence_summary(&json!({
            "puuid": "friend",
            "state": "mobile",
            "product": "league_of_legends",
            "private": ""
        }));
        assert_eq!(summary["state"], "offline");
        assert_eq!(summary["product"], "");
    }

    #[test]
    fn mobile_valorant_presence_stays_online() {
        let summary = presence_summary(&json!({
            "puuid": "friend",
            "state": "mobile",
            "product": "valorant",
            "private": ""
        }));
        assert_eq!(summary["state"], "mobile");
        assert_eq!(summary["product"], "valorant");
    }
}
