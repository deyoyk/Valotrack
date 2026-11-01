use std::{collections::HashMap, sync::Arc};

use serde_json::{json, Value};

use crate::app::{store, AppState};

pub fn record_board(state: &Arc<AppState>, board: &Value) {
    let state_name = board.get("state").and_then(Value::as_str).unwrap_or("");
    if state_name != "INGAME" && state_name != "PREGAME" {
        return;
    }
    let players = board
        .get("players")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(me) = players
        .iter()
        .find(|player| player.get("isSelf").and_then(Value::as_bool) == Some(true))
    else {
        return;
    };
    let Some(owner) = me.get("puuid").and_then(Value::as_str) else {
        return;
    };
    let self_team = board
        .get("selfTeam")
        .and_then(Value::as_str)
        .unwrap_or("Blue")
        .to_string();
    let match_id = board
        .get("matchId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if match_id.is_empty() || match_id == "lobby" {
        return;
    }
    let mut data = store::read(&state.data_dir, "encounters");
    if data.get("accounts").is_none() {
        data["accounts"] = json!({});
    }
    let Some(accounts) = data["accounts"].as_object_mut() else {
        return;
    };
    let account = accounts
        .entry(owner.to_string())
        .or_insert_with(|| json!({ "players": {}, "matches": {} }));
    {
        let Some(matches) = account.get_mut("matches").and_then(Value::as_object_mut) else {
            return;
        };
        if matches.contains_key(&match_id) {
            return;
        }
        matches.insert(
            match_id.clone(),
            json!({
                "ts": crate::history::now(),
                "map": board.get("map"),
                "state": state_name,
                "players": players.iter().filter_map(|player| player.get("puuid").cloned()).collect::<Vec<_>>(),
                "result": Value::Null
            }),
        );
    }
    {
        let Some(roster) = account.get_mut("players").and_then(Value::as_object_mut) else {
            return;
        };
        for player in &players {
            if player.get("isSelf").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            let Some(puuid) = player.get("puuid").and_then(Value::as_str) else {
                continue;
            };
            let entry = roster.entry(puuid.to_string()).or_insert_with(|| {
                json!({
                    "puuid": puuid,
                    "withCount": 0,
                    "againstCount": 0,
                    "winsWith": 0,
                    "lossesWith": 0,
                    "winsAgainst": 0,
                    "lossesAgainst": 0,
                    "matches": []
                })
            });
            let same_team = player.get("team").and_then(Value::as_str) == Some(self_team.as_str());
            let key = if same_team { "withCount" } else { "againstCount" };
            let count = entry.get(key).and_then(Value::as_i64).unwrap_or(0);
            entry[key] = json!(count + 1);
            entry["name"] = player.get("name").cloned().unwrap_or(Value::Null);
            entry["rankTier"] = player.get("rankTier").cloned().unwrap_or(Value::Null);
            entry["rank"] = player.get("rank").cloned().unwrap_or(Value::Null);
            entry["rankColor"] = player.get("rankColor").cloned().unwrap_or(Value::Null);
            entry["peakRank"] = player.get("peakRank").cloned().unwrap_or(Value::Null);
            entry["level"] = player.get("level").cloned().unwrap_or(Value::Null);
            entry["kd"] = player.get("kd").cloned().unwrap_or(Value::Null);
            entry["winRate"] = player.get("winRate").cloned().unwrap_or(Value::Null);
            entry["agent"] = player.get("agent").cloned().unwrap_or(Value::Null);
            entry["agentPortrait"] = player.get("agentPortrait").cloned().unwrap_or(Value::Null);
            entry["agentColor"] = player.get("agentColor").cloned().unwrap_or(Value::Null);
            entry["lastSeen"] = json!(crate::history::now());
            entry["lastMatch"] = json!(match_id);
            if let Some(list) = entry.get_mut("matches").and_then(Value::as_array_mut) {
                if !list.iter().any(|row| row.as_str() == Some(match_id.as_str())) {
                    list.push(json!(match_id));
                    if list.len() > 100 {
                        list.remove(0);
                    }
                }
            }
        }
    }
    let score = board.get("score").cloned().unwrap_or(Value::Null);
    let Some(ally) = score.get("ally").and_then(Value::as_i64) else {
        let _ = store::write(&state.data_dir, "encounters", &data);
        return;
    };
    let Some(enemy) = score.get("enemy").and_then(Value::as_i64) else {
        let _ = store::write(&state.data_dir, "encounters", &data);
        return;
    };
    if (ally >= 13 || enemy >= 13) && (ally - enemy).abs() >= 2 {
        let won = ally > enemy;
        {
            if let Some(record) = account
                .get_mut("matches")
                .and_then(Value::as_object_mut)
                .and_then(|matches| matches.get_mut(&match_id))
            {
                record["result"] = json!(if won { "Victory" } else { "Defeat" });
            }
        }
        if let Some(roster) = account.get_mut("players").and_then(Value::as_object_mut) {
            for player in &players {
                if player.get("isSelf").and_then(Value::as_bool) == Some(true) {
                    continue;
                }
                let Some(puuid) = player.get("puuid").and_then(Value::as_str) else {
                    continue;
                };
                let same_team =
                    player.get("team").and_then(Value::as_str) == Some(self_team.as_str());
                if let Some(entry) = roster.get_mut(puuid) {
                    let key = if same_team {
                        if won {
                            "winsWith"
                        } else {
                            "lossesWith"
                        }
                    } else if won {
                        "winsAgainst"
                    } else {
                        "lossesAgainst"
                    };
                    let current = entry.get(key).and_then(Value::as_i64).unwrap_or(0);
                    entry[key] = json!(current + 1);
                }
            }
        }
    }
    let _ = store::write(&state.data_dir, "encounters", &data);
}

pub fn list(state: &Arc<AppState>, owner: Option<&str>, scope_all: bool) -> Value {
    let data = store::read(&state.data_dir, "encounters");
    let accounts = data
        .get("accounts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let selected: Vec<&Value> = if scope_all {
        accounts.values().collect()
    } else if let Some(owner) = owner {
        accounts.get(owner).map(|value| vec![value]).unwrap_or_default()
    } else {
        accounts.values().collect()
    };
    let mut rows: HashMap<String, Value> = HashMap::new();
    for account in selected {
        let Some(players) = account.get("players").and_then(Value::as_object) else {
            continue;
        };
        for (puuid, player) in players {
            let entry = rows.entry(puuid.clone()).or_insert_with(|| {
                let mut base = player.clone();
                for key in [
                    "withCount",
                    "againstCount",
                    "winsWith",
                    "lossesWith",
                    "winsAgainst",
                    "lossesAgainst",
                ] {
                    base[key] = json!(0);
                }
                base
            });
            for key in [
                "withCount",
                "againstCount",
                "winsWith",
                "lossesWith",
                "winsAgainst",
                "lossesAgainst",
            ] {
                let current = entry.get(key).and_then(Value::as_i64).unwrap_or(0);
                let add = player.get(key).and_then(Value::as_i64).unwrap_or(0);
                entry[key] = json!(current + add);
            }
            if player
                .get("lastSeen")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > entry.get("lastSeen").and_then(Value::as_u64).unwrap_or(0)
            {
                entry["lastSeen"] = player.get("lastSeen").cloned().unwrap_or(Value::Null);
                entry["name"] = player.get("name").cloned().unwrap_or(Value::Null);
                entry["rank"] = player.get("rank").cloned().unwrap_or(Value::Null);
                entry["agent"] = player.get("agent").cloned().unwrap_or(Value::Null);
            }
        }
    }
    let mut players: Vec<Value> = rows.into_values().collect();
    players.sort_by_key(|player| {
        let with = player.get("withCount").and_then(Value::as_i64).unwrap_or(0);
        let against = player.get("againstCount").and_then(Value::as_i64).unwrap_or(0);
        -with.max(against)
    });
    players.truncate(500);
    json!({
        "players": players,
        "accountCount": accounts.len(),
        "scope": if scope_all { "all" } else { "current" }
    })
}

pub fn one(state: &Arc<AppState>, owner: Option<&str>, puuid: &str) -> Value {
    let data = store::read(&state.data_dir, "encounters");
    let accounts = data
        .get("accounts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut out = Value::Null;
    for (account_id, account) in accounts {
        if owner.is_some() && owner != Some(account_id.as_str()) {
            continue;
        }
        if let Some(player) = account
            .get("players")
            .and_then(Value::as_object)
            .and_then(|players| players.get(puuid))
        {
            out = player.clone();
            break;
        }
    }
    out
}
