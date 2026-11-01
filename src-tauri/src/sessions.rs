use std::sync::Arc;

use serde_json::{json, Value};

use crate::{
    app::{store, AppState},
    history,
};

pub fn list(state: &Arc<AppState>) -> Value {
    let payload = history::payload(state, None);
    let owner = payload
        .get("owner")
        .and_then(Value::as_str)
        .unwrap_or("");
    let data = store::read(&state.data_dir, "sessions");
    let sessions = data
        .get("accounts")
        .and_then(Value::as_object)
        .and_then(|accounts| accounts.get(owner))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let points = payload
        .get("points")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut views: Vec<Value> = sessions
        .iter()
        .map(|session| view(session, &points))
        .collect();
    views.sort_by_key(|session| {
        -session
            .get("startedAt")
            .and_then(Value::as_i64)
            .unwrap_or(0)
    });
    let active = views
        .iter()
        .find(|session| session.get("active").and_then(Value::as_bool) == Some(true))
        .cloned();
    json!({
        "owner": owner,
        "active": active,
        "sessions": views
    })
}

pub fn start(
    state: &Arc<AppState>,
    goal_type: Option<&str>,
    target: Option<i64>,
) -> Result<Value, String> {
    let payload = history::payload(state, None);
    let owner = payload
        .get("owner")
        .and_then(Value::as_str)
        .filter(|owner| !owner.is_empty())
        .ok_or_else(|| "No saved account yet. Open VALORANT and play a ranked game first.".to_string())?;
    let summary = payload
        .get("summary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let goal_type = match goal_type.unwrap_or("games") {
        "wins" => "wins",
        "rr" => "rr",
        _ => "games",
    };
    let target = target.unwrap_or(5).clamp(1, 100);
    let mut data = store::read(&state.data_dir, "sessions");
    if data.get("accounts").is_none() {
        data["accounts"] = json!({});
    }
    let accounts = data["accounts"]
        .as_object_mut()
        .ok_or("session store is invalid")?;
    let sessions = accounts
        .entry(owner.to_string())
        .or_insert_with(|| json!([]));
    let Some(rows) = sessions.as_array_mut() else {
        return Err("session store is invalid".into());
    };
    for session in rows.iter_mut() {
        if session.get("active").and_then(Value::as_bool) == Some(true) {
            session["active"] = json!(false);
            session["endedAt"] = json!(history::now());
        }
    }
    let session = json!({
        "id": format!("s-{}", history::now()),
        "goalType": goal_type,
        "target": target,
        "startedAt": history::now(),
        "endedAt": Value::Null,
        "active": true,
        "baseline": {
            "games": summary.get("games").and_then(Value::as_i64).unwrap_or(0),
            "wins": summary.get("wins").and_then(Value::as_i64).unwrap_or(0),
            "netRr": summary.get("netRr").and_then(Value::as_i64).unwrap_or(0)
        }
    });
    rows.push(session.clone());
    store::write(&state.data_dir, "sessions", &data)?;
    Ok(json!({ "ok": true, "session": view(&session, &payload.get("points").and_then(Value::as_array).cloned().unwrap_or_default()) }))
}

pub fn end(state: &Arc<AppState>) -> Result<Value, String> {
    let payload = history::payload(state, None);
    let owner = payload
        .get("owner")
        .and_then(Value::as_str)
        .ok_or_else(|| "No active account.".to_string())?;
    let mut data = store::read(&state.data_dir, "sessions");
    let Some(rows) = data
        .get_mut("accounts")
        .and_then(Value::as_object_mut)
        .and_then(|accounts| accounts.get_mut(owner))
        .and_then(Value::as_array_mut)
    else {
        return Err("No sessions to end.".into());
    };
    let now = history::now();
    let mut found = None;
    for session in rows.iter_mut() {
        if session.get("active").and_then(Value::as_bool) == Some(true) {
            session["active"] = json!(false);
            session["endedAt"] = json!(now);
            found = Some(session.clone());
        }
    }
    let session = found.ok_or_else(|| "No active session.".to_string())?;
    store::write(&state.data_dir, "sessions", &data)?;
    let points = payload
        .get("points")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(json!({ "ok": true, "session": view(&session, &points) }))
}

pub fn delete(state: &Arc<AppState>, id: &str) -> Result<Value, String> {
    let payload = history::payload(state, None);
    let owner = payload
        .get("owner")
        .and_then(Value::as_str)
        .ok_or_else(|| "No saved account.".to_string())?;
    let mut data = store::read(&state.data_dir, "sessions");
    let Some(rows) = data
        .get_mut("accounts")
        .and_then(Value::as_object_mut)
        .and_then(|accounts| accounts.get_mut(owner))
        .and_then(Value::as_array_mut)
    else {
        return Err("Session not found.".into());
    };
    rows.retain(|session| session.get("id").and_then(Value::as_str) != Some(id));
    store::write(&state.data_dir, "sessions", &data)?;
    Ok(json!({ "ok": true }))
}

fn view(session: &Value, points: &[Value]) -> Value {
    let started = session
        .get("startedAt")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let active = session.get("active").and_then(Value::as_bool) == Some(true);
    let relevant: Vec<&Value> = points
        .iter()
        .filter(|point| point.get("ts").and_then(Value::as_i64).unwrap_or(0) >= started)
        .collect();
    let games = relevant.len() as i64;
    let wins = relevant
        .iter()
        .filter(|point| point.get("result").and_then(Value::as_str) == Some("Victory"))
        .count() as i64;
    let losses = relevant
        .iter()
        .filter(|point| point.get("result").and_then(Value::as_str) == Some("Defeat"))
        .count() as i64;
    let net_rr: i64 = relevant
        .iter()
        .map(|point| point.get("delta").and_then(Value::as_i64).unwrap_or(0))
        .sum();
    let goal_type = session
        .get("goalType")
        .and_then(Value::as_str)
        .unwrap_or("games");
    let target = session.get("target").and_then(Value::as_i64).unwrap_or(5);
    let current = match goal_type {
        "wins" => wins,
        "rr" => net_rr,
        _ => games,
    };
    let progress = if target <= 0 {
        0.0
    } else {
        (current as f64 / target as f64).clamp(0.0, 1.0) * 100.0
    };
    json!({
        "id": session.get("id"),
        "goalType": goal_type,
        "target": target,
        "startedAt": started,
        "endedAt": session.get("endedAt"),
        "active": active,
        "games": games,
        "wins": wins,
        "losses": losses,
        "winRate": if games > 0 { (100.0 * wins as f64 / games as f64).round() } else { 0.0 },
        "netRr": net_rr,
        "current": current,
        "progress": progress.round(),
        "complete": current >= target
    })
}
