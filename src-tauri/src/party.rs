use std::{collections::HashMap, sync::Arc};

use serde_json::{json, Value};

use crate::{app::AppState, content, riot::Session};

async fn current_party_id(session: &Session) -> Option<String> {
    session
        .glz_get(&format!("/parties/v1/players/{}", session.puuid))
        .await
        .ok()
        .and_then(|value| {
            value
                .get("CurrentPartyID")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|party_id| !party_id.is_empty())
}

pub async fn details(state: &Arc<AppState>) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let Some(party_id) = current_party_id(&session).await else {
        return Ok(json!({ "available": false, "message": "Not in a party" }));
    };
    let party = session
        .glz_get(&format!("/parties/v1/parties/{party_id}"))
        .await
        .map_err(|error| format!("Party unavailable: {error}"))?;
    let titles = state.valo.titles().await;
    let self_label = content::account_label(&session).await;
    let (self_name, self_tag) = self_label
        .split_once('#')
        .map(|(name, tag)| (name.to_string(), tag.to_string()))
        .unwrap_or((self_label.clone(), String::new()));
    let mut names: HashMap<String, (String, String)> = HashMap::new();
    for presence in session.presences().await.unwrap_or_default() {
        if presence.get("product").and_then(Value::as_str) != Some("riot_client") {
            continue;
        }
        let Some(puuid) = presence.get("puuid").and_then(Value::as_str) else {
            continue;
        };
        let name = presence.get("game_name").and_then(Value::as_str).unwrap_or("");
        let tag = presence.get("game_tag").and_then(Value::as_str).unwrap_or("");
        if !name.is_empty() {
            names.insert(puuid.to_string(), (name.to_string(), tag.to_string()));
        }
    }
    let members = party
        .get("Members")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|member| {
                    let subject = member.get("Subject").and_then(Value::as_str).unwrap_or("");
                    let identity = member
                        .get("PlayerIdentity")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    let card_id = identity
                        .get("PlayerCardID")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let title_id = identity
                        .get("PlayerTitleID")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_lowercase();
                    let (name, tag) = if subject == session.puuid {
                        (self_name.clone(), self_tag.clone())
                    } else {
                        names
                            .get(subject)
                            .cloned()
                            .unwrap_or_else(|| (subject.chars().take(8).collect(), String::new()))
                    };
                    json!({
                        "puuid": subject,
                        "name": name,
                        "tag": tag,
                        "isReady": member.get("IsReady").and_then(Value::as_bool).unwrap_or(false),
                        "isOwner": member.get("IsOwner").and_then(Value::as_bool).unwrap_or(false),
                        "tier": member.get("CompetitiveTier").and_then(Value::as_i64).unwrap_or(0),
                        "accountLevel": identity.get("AccountLevel").and_then(Value::as_i64).unwrap_or(0),
                        "incognito": identity.get("Incognito").and_then(Value::as_bool).unwrap_or(false),
                        "card": if card_id.is_empty() { Value::Null } else { json!(format!("https://media.valorant-api.com/playercards/{card_id}/displayicon.png")) },
                        "title": titles.get(&title_id).cloned().unwrap_or_default(),
                        "isSelf": subject == session.puuid
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(json!({
        "available": true,
        "partyId": party_id,
        "accessibility": party.get("Accessibility").and_then(Value::as_str).unwrap_or("CLOSED"),
        "inviteCode": party.get("InviteCode").and_then(Value::as_str).unwrap_or(""),
        "state": party.get("State").and_then(Value::as_str).unwrap_or("DEFAULT"),
        "queueId": party.get("MatchmakingData").and_then(|value| value.get("QueueID")).and_then(Value::as_str).unwrap_or(""),
        "eligible": party.get("EligibleQueues").cloned().unwrap_or_else(|| json!([])),
        "members": members,
        "selfReady": members.iter().find(|member| member.get("isSelf").and_then(Value::as_bool) == Some(true)).and_then(|member| member.get("isReady")).cloned().unwrap_or(json!(false)),
        "isOwner": members.iter().find(|member| member.get("isSelf").and_then(Value::as_bool) == Some(true)).and_then(|member| member.get("isOwner")).cloned().unwrap_or(json!(false))
    }))
}

async fn party_and_session(state: &Arc<AppState>) -> Result<(Arc<Session>, String), String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let party_id = current_party_id(&session)
        .await
        .ok_or_else(|| "Not in a party.".to_string())?;
    Ok((session, party_id))
}

pub async fn set_ready(
    state: &Arc<AppState>,
    ready: bool,
    dry_run: bool,
) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    if dry_run || !settings.allow_live_actions {
        return Ok(json!({
            "ok": true,
            "status": "dry-run",
            "message": format!("DRY-RUN: would mark you as {}.", if ready { "ready" } else { "not ready" })
        }));
    }
    let (session, party_id) = party_and_session(state).await?;
    let response = session
        .glz_post(
            &format!(
                "/parties/v1/parties/{party_id}/members/{}/setReady",
                session.puuid
            ),
            Some(&json!({ "ready": ready })),
        )
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Riot refused the ready change (HTTP {}).", response.status()));
    }
    Ok(json!({
        "ok": true,
        "status": "updated",
        "message": if ready { "You are ready." } else { "You are no longer ready." }
    }))
}

pub async fn set_accessibility(
    state: &Arc<AppState>,
    open: bool,
    dry_run: bool,
) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    if dry_run || !settings.allow_live_actions {
        return Ok(json!({
            "ok": true,
            "status": "dry-run",
            "message": format!("DRY-RUN: would set the party to {}.", if open { "open" } else { "closed" })
        }));
    }
    let (session, party_id) = party_and_session(state).await?;
    let response = session
        .glz_post(
            &format!("/parties/v1/parties/{party_id}/accessibility"),
            Some(&json!({ "accessibility": if open { "OPEN" } else { "CLOSED" } })),
        )
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Riot refused the accessibility change (HTTP {}).", response.status()));
    }
    Ok(json!({
        "ok": true,
        "status": "updated",
        "message": if open { "Party is open." } else { "Party is closed." }
    }))
}

pub async fn invite(
    state: &Arc<AppState>,
    name: &str,
    tag: &str,
    dry_run: bool,
) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    if dry_run || !settings.allow_live_actions {
        return Ok(json!({
            "ok": true,
            "status": "dry-run",
            "message": format!("DRY-RUN: would invite {name}#{tag}.")
        }));
    }
    let (session, party_id) = party_and_session(state).await?;
    let response = session
        .glz_post(
            &format!(
                "/parties/v1/parties/{party_id}/invites/name/{}/tag/{}",
                urlencoding::encode(name),
                urlencoding::encode(tag)
            ),
            None,
        )
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Riot refused the invite (HTTP {}).", response.status()));
    }
    Ok(json!({
        "ok": true,
        "status": "invited",
        "message": format!("Invited {name}#{tag}.")
    }))
}

pub async fn generate_code(state: &Arc<AppState>, dry_run: bool) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    if dry_run || !settings.allow_live_actions {
        return Ok(json!({
            "ok": true,
            "status": "dry-run",
            "message": "DRY-RUN: would generate a party invite code."
        }));
    }
    let (session, party_id) = party_and_session(state).await?;
    let response = session
        .glz_post(&format!("/parties/v1/parties/{party_id}/invitecode"), None)
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Riot refused the code request (HTTP {}).", response.status()));
    }
    let party = response.json::<Value>().await.unwrap_or_else(|_| json!({}));
    Ok(json!({
        "ok": true,
        "status": "generated",
        "message": "Party invite code generated.",
        "code": party.get("InviteCode").and_then(Value::as_str).unwrap_or("")
    }))
}

pub async fn join_code(
    state: &Arc<AppState>,
    code: &str,
    dry_run: bool,
) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    if dry_run || !settings.allow_live_actions {
        return Ok(json!({
            "ok": true,
            "status": "dry-run",
            "message": format!("DRY-RUN: would join party code {code}.")
        }));
    }
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let response = session
        .glz_post(
            &format!(
                "/parties/v1/players/joinbycode/{}",
                urlencoding::encode(code)
            ),
            None,
        )
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Riot refused the party code (HTTP {}).", response.status()));
    }
    Ok(json!({
        "ok": true,
        "status": "joined",
        "message": "Joined the party."
    }))
}
