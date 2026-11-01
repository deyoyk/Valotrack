use std::{collections::HashMap, sync::Arc, time::{SystemTime, UNIX_EPOCH}};

use chrono::{Datelike, Local, TimeZone, Timelike};
use serde_json::{json, Value};

use crate::{app::{store, AppState}, catalog, riot::Session};

const MAX_POINTS: usize = 2000;

pub async fn refresh(state: &Arc<AppState>, session: &Session) -> Result<(), String> {
    let puuid = session.puuid.clone();
    let cache_key = format!("history-refresh:{puuid}");
    if let Some(at) = state.meta.lock().get(&cache_key).and_then(Value::as_u64) {
        if now() - at < 600 {
            return Ok(());
        }
    }
    state
        .meta
        .lock()
        .insert(cache_key, json!(now()));
    let mut updates = Vec::new();
    for start in [0, 20] {
        let data = session
            .pd_get(&format!(
                "/mmr/v1/players/{puuid}/competitiveupdates?startIndex={start}&endIndex={}&queue=competitive",
                start + 20
            ))
            .await
            .map_err(|error| error.to_string())?;
        let page = data
            .get("Matches")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let complete = page.len() >= 20;
        updates.extend(page);
        if !complete {
            break;
        }
    }
    if updates.is_empty() {
        return Ok(());
    }
    let mut data = store::read(&state.data_dir, "history");
    if data.get("accounts").is_none() {
        data["accounts"] = json!({});
    }
    let account = data["accounts"]
        .as_object_mut()
        .ok_or("history store is invalid")?
        .entry(puuid.clone())
        .or_insert_with(|| json!({ "points": [] }));
    account["lastSeenAt"] = json!(now());
    let mut points = account
        .get("points")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in updates {
        let Some(match_id) = row.get("MatchID").and_then(Value::as_str) else {
            continue;
        };
        let ts = row.get("MatchStartTime").and_then(Value::as_i64).unwrap_or(0) / 1000;
        if ts <= 0 {
            continue;
        }
        let delta = row.get("RankedRatingEarned").and_then(Value::as_i64).unwrap_or(0);
        let result = if delta > 0 {
            "Victory"
        } else if delta < 0 {
            "Defeat"
        } else {
            "Draw"
        };
        let point = json!({
            "matchId": match_id,
            "ts": ts,
            "map": catalog::map_name_from_id(row.get("MapID").and_then(Value::as_str).unwrap_or("")),
            "result": result,
            "resultExact": false,
            "delta": delta,
            "tier": row.get("TierAfterUpdate").and_then(Value::as_i64).unwrap_or(0),
            "rr": row.get("RankedRatingAfterUpdate").and_then(Value::as_i64).unwrap_or(0),
            "seasonId": row.get("SeasonID").or_else(|| row.get("SeasonId")).cloned().unwrap_or(Value::Null)
        });
        if let Some(existing) = points
            .iter_mut()
            .find(|point| point.get("matchId").and_then(Value::as_str) == Some(match_id))
        {
            *existing = point;
        } else {
            points.push(point);
        }
    }
    points.sort_by_key(|point| point.get("ts").and_then(Value::as_i64).unwrap_or(0));
    if points.len() > MAX_POINTS {
        points.drain(0..points.len() - MAX_POINTS);
    }
    account["points"] = json!(points);
    store::write(&state.data_dir, "history", &data)
}

pub fn remember_account(state: &Arc<AppState>, puuid: &str, name: Option<&str>) {
    if puuid.is_empty() {
        return;
    }
    let mut data = store::read(&state.data_dir, "history");
    if data.get("accounts").is_none() {
        data["accounts"] = json!({});
    }
    if let Some(accounts) = data["accounts"].as_object_mut() {
        let account = accounts.entry(puuid.to_string()).or_insert_with(|| json!({}));
        if let Some(name) = name {
            if name.contains('#') {
                account["riotId"] = json!(name);
            }
        }
        account["lastSeenAt"] = json!(now());
    }
    let _ = store::write(&state.data_dir, "history", &data);
}

pub fn payload(state: &Arc<AppState>, puuid: Option<&str>) -> Value {
    let data = store::read(&state.data_dir, "history");
    let accounts = data
        .get("accounts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let owner = puuid
        .filter(|puuid| accounts.contains_key(*puuid))
        .map(str::to_string)
        .or_else(|| {
            accounts
                .iter()
                .max_by_key(|(_, account)| {
                    account.get("lastSeenAt").and_then(Value::as_u64).unwrap_or(0)
                })
                .map(|(puuid, _)| puuid.clone())
        });
    let points = owner
        .as_ref()
        .and_then(|owner| accounts.get(owner))
        .and_then(|account| account.get("points"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let account_rows: Vec<Value> = accounts
        .iter()
        .map(|(puuid, account)| {
            json!({
                "puuid": puuid,
                "riotId": account.get("riotId"),
                "matches": account.get("points").and_then(Value::as_array).map(Vec::len).unwrap_or(0),
                "lastSeenAt": account.get("lastSeenAt").and_then(Value::as_u64).unwrap_or(0)
            })
        })
        .collect();
    let summary = summary(&points);
    let insights = insights(&points);
    let sessions = sessions(&points);
    let recent: Vec<Value> = points.iter().rev().take(20).cloned().collect();
    let last = points.last().cloned().unwrap_or_else(|| json!({}));
    let tier = last.get("tier").and_then(Value::as_i64).unwrap_or(0) as i32;
    let rank = catalog::rank_from_tier(tier);
    json!({
        "owner": owner,
        "accounts": account_rows,
        "points": points,
        "recent": recent,
        "summary": summary,
        "insights": insights,
        "sessions": sessions,
        "rank": rank.name,
        "rankColor": rank.color,
        "rankTier": rank.tier,
        "rr": last.get("rr").and_then(Value::as_i64).unwrap_or(0),
        "refreshing": false
    })
}

fn summary(points: &[Value]) -> Value {
    let games = points.len();
    if games == 0 {
        return json!({
            "games": 0,
            "wins": 0,
            "losses": 0,
            "winRate": 0,
            "avgDelta": 0,
            "netRr": 0,
            "kd": Value::Null,
            "hsPct": Value::Null,
            "maps": [],
            "streak": Value::Null,
            "tilt": Value::Null
        });
    }
    let wins = points
        .iter()
        .filter(|point| point.get("result").and_then(Value::as_str) == Some("Victory"))
        .count();
    let losses = points
        .iter()
        .filter(|point| point.get("result").and_then(Value::as_str) == Some("Defeat"))
        .count();
    let deltas: Vec<i64> = points
        .iter()
        .filter_map(|point| point.get("delta").and_then(Value::as_i64))
        .collect();
    let net: i64 = deltas.iter().sum();
    let mut by_map: HashMap<String, (i64, i64, i64)> = HashMap::new();
    for point in points {
        let map = point
            .get("map")
            .and_then(Value::as_str)
            .unwrap_or("Unknown")
            .to_string();
        let entry = by_map.entry(map).or_default();
        entry.0 += 1;
        if point.get("result").and_then(Value::as_str) == Some("Victory") {
            entry.1 += 1;
        }
        entry.2 += point.get("delta").and_then(Value::as_i64).unwrap_or(0);
    }
    let mut maps: Vec<Value> = by_map
        .into_iter()
        .map(|(map, (games, wins, delta))| {
            json!({
                "map": map,
                "games": games,
                "wins": wins,
                "winRate": (100.0 * wins as f64 / games as f64).round(),
                "delta": delta
            })
        })
        .collect();
    maps.sort_by_key(|row| -row.get("games").and_then(Value::as_i64).unwrap_or(0));
    let streak = streak(points);
    let tilt = tilt(points);
    json!({
        "games": games,
        "wins": wins,
        "losses": losses,
        "winRate": (100.0 * wins as f64 / games as f64).round(),
        "avgDelta": if deltas.is_empty() { 0.0 } else { (deltas.iter().sum::<i64>() as f64 / deltas.len() as f64 * 10.0).round() / 10.0 },
        "netRr": net,
        "maps": maps,
        "streak": streak,
        "tilt": tilt
    })
}

fn streak(points: &[Value]) -> Value {
    let mut iter = points.iter().rev();
    let Some(last) = iter.next() else {
        return Value::Null;
    };
    let result = last.get("result").and_then(Value::as_str).unwrap_or("Draw");
    if result == "Draw" {
        return Value::Null;
    }
    let count = 1 + iter
        .take_while(|point| point.get("result").and_then(Value::as_str) == Some(result))
        .count();
    json!({ "type": result, "count": count })
}

fn tilt(points: &[Value]) -> Value {
    let recent: Vec<&Value> = points.iter().rev().take(5).collect();
    if recent.len() < 3 {
        return Value::Null;
    }
    let losses = recent
        .iter()
        .filter(|point| point.get("result").and_then(Value::as_str) == Some("Defeat"))
        .count();
    let net: i64 = recent
        .iter()
        .map(|point| point.get("delta").and_then(Value::as_i64).unwrap_or(0))
        .sum();
    if losses >= 4 {
        return json!({
            "active": true,
            "level": "high",
            "message": format!("{losses} losses in the last {} games. Take a break.", recent.len())
        });
    }
    if net <= -60 {
        return json!({
            "active": true,
            "level": "medium",
            "message": format!("Down {net} RR across the last {} games.", recent.len())
        });
    }
    if losses >= 3 {
        return json!({
            "active": true,
            "level": "low",
            "message": format!("{losses} losses in the last {} games.", recent.len())
        });
    }
    json!({ "active": false, "level": "none", "message": "No tilt signals in the last 5 games." })
}

fn insights(points: &[Value]) -> Vec<Value> {
    let rated: Vec<&Value> = points
        .iter()
        .filter(|point| {
            matches!(
                point.get("result").and_then(Value::as_str),
                Some("Victory") | Some("Defeat")
            )
        })
        .collect();
    if rated.len() < 20 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut parts: HashMap<&str, (i64, i64)> = HashMap::new();
    let mut weekends = (0i64, 0i64);
    let mut weekdays = (0i64, 0i64);
    for point in &rated {
        let Some(ts) = point.get("ts").and_then(Value::as_i64) else {
            continue;
        };
        let Some(dt) = Local.timestamp_opt(ts, 0).single() else {
            continue;
        };
        let part = match dt.hour() {
            5..=11 => "morning",
            12..=16 => "afternoon",
            17..=21 => "evening",
            _ => "night",
        };
        let entry = parts.entry(part).or_default();
        entry.1 += 1;
        if point.get("result").and_then(Value::as_str) == Some("Victory") {
            entry.0 += 1;
        }
        let target = if dt.weekday().num_days_from_monday() >= 5 {
            &mut weekends
        } else {
            &mut weekdays
        };
        target.1 += 1;
        if point.get("result").and_then(Value::as_str) == Some("Victory") {
            target.0 += 1;
        }
    }
    let mut buckets: Vec<(&str, i64)> = parts
        .into_iter()
        .filter(|(_, (_, games))| *games >= 8)
        .map(|(part, (wins, games))| (part, (100.0 * wins as f64 / games as f64).round() as i64))
        .collect();
    buckets.sort_by_key(|(_, rate)| *rate);
    if buckets.len() >= 2 {
        let (worst, worst_rate) = buckets[0];
        let (best, best_rate) = *buckets.last().unwrap();
        if best_rate - worst_rate >= 10 {
            out.push(json!({
                "title": "Time of day",
                "text": format!("{best} games are {best_rate}% wins, compared with {worst_rate}% in the {worst}."),
                "tone": if best_rate >= 50 { "pos" } else { "neutral" }
            }));
        }
    }
    if weekends.1 >= 8 && weekdays.1 >= 8 {
        let weekend_rate = (100.0 * weekends.0 as f64 / weekends.1 as f64).round() as i64;
        let weekday_rate = (100.0 * weekdays.0 as f64 / weekdays.1 as f64).round() as i64;
        if (weekend_rate - weekday_rate).abs() >= 10 {
            let better = if weekend_rate > weekday_rate { "Weekends" } else { "Weekdays" };
            out.push(json!({
                "title": "Weekly rhythm",
                "text": format!("{better} are your stronger stretch ({weekend_rate}% vs {weekday_rate}%)."),
                "tone": "neutral"
            }));
        }
    }
    let summary = summary(points);
    if let Some(maps) = summary.get("maps").and_then(Value::as_array) {
        let eligible: Vec<&Value> = maps
            .iter()
            .filter(|row| row.get("games").and_then(Value::as_i64).unwrap_or(0) >= 5)
            .collect();
        if let Some(best) = eligible
            .iter()
            .max_by_key(|row| row.get("winRate").and_then(Value::as_i64).unwrap_or(0))
        {
            out.push(json!({
                "title": "Best map",
                "text": format!(
                    "{} is {}% across {} games.",
                    best.get("map").and_then(Value::as_str).unwrap_or("Unknown"),
                    best.get("winRate").and_then(Value::as_i64).unwrap_or(0),
                    best.get("games").and_then(Value::as_i64).unwrap_or(0)
                ),
                "tone": "pos"
            }));
        }
    }
    out
}

fn sessions(points: &[Value]) -> Vec<Value> {
    let mut groups: Vec<Vec<&Value>> = Vec::new();
    let mut current: Vec<&Value> = Vec::new();
    let mut last_ts = 0i64;
    for point in points {
        let ts = point.get("ts").and_then(Value::as_i64).unwrap_or(0);
        if !current.is_empty() && ts - last_ts > 2 * 3600 {
            groups.push(std::mem::take(&mut current));
        }
        current.push(point);
        last_ts = ts;
    }
    if !current.is_empty() {
        groups.push(current);
    }
    let mut rows: Vec<Value> = groups
        .into_iter()
        .map(|group| {
            let wins = group
                .iter()
                .filter(|point| point.get("result").and_then(Value::as_str) == Some("Victory"))
                .count();
            let start = group
                .first()
                .and_then(|point| point.get("ts"))
                .cloned()
                .unwrap_or(Value::Null);
            let end = group
                .last()
                .and_then(|point| point.get("ts"))
                .cloned()
                .unwrap_or(Value::Null);
            let delta: i64 = group
                .iter()
                .map(|point| point.get("delta").and_then(Value::as_i64).unwrap_or(0))
                .sum();
            json!({
                "id": format!("session-{}", start),
                "start": start,
                "end": end,
                "games": group.len(),
                "wins": wins,
                "losses": group.len() - wins,
                "winRate": (100.0 * wins as f64 / group.len() as f64).round(),
                "netRr": delta
            })
        })
        .collect();
    rows.sort_by_key(|row| -row.get("start").and_then(Value::as_i64).unwrap_or(0));
    rows
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
