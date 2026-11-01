use std::{collections::HashMap, sync::Arc, time::Instant};

use futures::{stream, StreamExt};
use serde_json::{json, Value};

use crate::{app::AppState, catalog, riot::Session};

pub async fn profile(state: &Arc<AppState>, puuid: &str) -> Result<Value, String> {
    let key = format!("career:{puuid}");
    if let Some(cached) = state.cache_json(&key) {
        return Ok(cached);
    }
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let mut value = career(&session, puuid, 8).await?;
    if let Some(matches) = value.get_mut("matches").and_then(Value::as_array_mut) {
        for row in matches {
            let map = row.get("map").and_then(Value::as_str).unwrap_or("");
            row["mapSplash"] = json!(state.valo.map_splash(map).await);
        }
    }
    value["encounter"] = crate::encounters::one(state, None, puuid);
    state.put_json(key, value.clone());
    Ok(value)
}

pub async fn career(
    session: &Session,
    puuid: &str,
    count: usize,
) -> Result<Value, String> {
    let history = session
        .pd_get(&format!(
            "/match-history/v1/history/{puuid}?startIndex=0&endIndex={count}"
        ))
        .await
        .map_err(|error| error.to_string())?;
    let mids: Vec<String> = history
        .get("History")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| entry.get("MatchID").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let details = stream::iter(mids.iter().cloned())
        .map(|mid| {
            let session = session.clone();
            async move { session.match_detail(&mid).await.ok().map(|detail| (mid, detail)) }
        })
        .buffer_unordered(4)
        .filter_map(|row| async move { row })
        .collect::<Vec<_>>()
        .await;
    let mut rows = Vec::new();
    let mut mate_ids = Vec::new();
    for (mid, detail) in &details {
        if let Some(row) = career_match(detail, puuid, mid) {
            for key in ["teammates", "opponents"] {
                mate_ids.extend(
                    row.get(key)
                        .and_then(Value::as_array)
                        .map(|mates| {
                            mates
                                .iter()
                                .filter_map(|mate| mate.get("puuid").and_then(Value::as_str))
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default(),
                );
            }
            rows.push(row);
        }
    }
    let names = session.reveal_names(&mate_ids).await;
    for row in &mut rows {
        for key in ["teammates", "opponents"] {
            if let Some(list) = row.get_mut(key).and_then(Value::as_array_mut) {
                for mate in list {
                    let puuid = mate.get("puuid").and_then(Value::as_str).unwrap_or("");
                    if let Some(name) = names.get(puuid) {
                        mate["name"] = json!(name);
                    }
                }
            }
        }
    }
    let updates = fetch_rr_updates(session, puuid, count).await;
    for row in &mut rows {
        if let Some(match_id) = row.get("matchId").and_then(Value::as_str) {
            if let Some(update) = updates.get(match_id) {
                let tier = update
                    .get("TierAfterUpdate")
                    .and_then(Value::as_i64)
                    .unwrap_or(0) as i32;
                let rank = catalog::rank_from_tier(tier);
                row["rrDelta"] = update
                    .get("RankedRatingEarned")
                    .cloned()
                    .unwrap_or(Value::Null);
                row["tierAfter"] = json!(tier);
                row["rrAfter"] = update
                    .get("RankedRatingAfterUpdate")
                    .cloned()
                    .unwrap_or(Value::Null);
                row["rankAfter"] = json!(rank.name);
                row["rankColor"] = json!(rank.color);
            }
        }
    }
    let summary = career_summary(&rows);
    Ok(json!({
        "source": "local",
        "puuid": puuid,
        "matches": rows,
        "averages": summary.get("averages"),
        "coPlayers": summary.get("coPlayers"),
        "agentPool": summary.get("agentPool"),
        "mapStats": summary.get("mapStats")
    }))
}

async fn fetch_rr_updates(
    session: &Session,
    puuid: &str,
    count: usize,
) -> HashMap<String, Value> {
    let updates = session
        .pd_get(&format!(
            "/mmr/v1/players/{puuid}/competitiveupdates?startIndex=0&endIndex={}&queue=competitive",
            count.clamp(10, 20)
        ))
        .await;
    let Ok(updates) = updates else {
        return HashMap::new();
    };
    updates
        .get("Matches")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    Some((
                        row.get("MatchID")?.as_str()?.to_string(),
                        row.clone(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn career_match(detail: &Value, puuid: &str, mid: &str) -> Option<Value> {
    let info = detail.get("matchInfo").cloned().unwrap_or_else(|| json!({}));
    let players = detail
        .get("players")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let me = players
        .iter()
        .find(|player| player.get("subject").and_then(Value::as_str) == Some(puuid))?;
    let stats = me.get("stats").cloned().unwrap_or_else(|| json!({}));
    let team_id = me.get("teamId").and_then(Value::as_str).unwrap_or("");
    let teams = detail
        .get("teams")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let my_team = teams
        .iter()
        .find(|team| team.get("teamId").and_then(Value::as_str) == Some(team_id))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let won = my_team.get("won").and_then(Value::as_bool);
    let rounds: i64 = teams
        .iter()
        .map(|team| team.get("roundsWon").and_then(Value::as_i64).unwrap_or(0))
        .sum();
    let (hits, heads) = headshot_totals(detail, puuid);
    let agent = me
        .get("characterId")
        .and_then(Value::as_str)
        .and_then(catalog::agent_name_by_uuid)
        .unwrap_or("Unknown");
    let opponent_score = teams
        .iter()
        .find(|team| team.get("teamId").and_then(Value::as_str) != Some(team_id))
        .and_then(|team| team.get("roundsWon"))
        .cloned()
        .unwrap_or(Value::Null);
    let kills = stats.get("kills").and_then(Value::as_i64).unwrap_or(0);
    let deaths = stats.get("deaths").and_then(Value::as_i64).unwrap_or(0);
    let mut teammates = Vec::new();
    let mut opponents = Vec::new();
    for player in &players {
        let Some(subject) = player.get("subject").and_then(Value::as_str) else {
            continue;
        };
        if subject == puuid || team_id.is_empty() || team_id == "Neutral" {
            continue;
        }
        let player_team = player.get("teamId").and_then(Value::as_str).unwrap_or("");
        if player_team == team_id || player_team.is_empty() {
        } else if player_team == "Neutral" {
            continue;
        }
        let pstats = player.get("stats").cloned().unwrap_or_else(|| json!({}));
        let agent_name = player
            .get("characterId")
            .and_then(Value::as_str)
            .and_then(catalog::agent_name_by_uuid)
            .unwrap_or("Unknown");
        let tier = player.get("competitiveTier").and_then(Value::as_i64).unwrap_or(0) as i32;
        let rank = catalog::rank_from_tier(tier);
        let row = json!({
            "puuid": subject,
            "name": player.get("gameName").and_then(Value::as_str).filter(|name| !name.is_empty())
                .map(|name| format!("{name}#{}", player.get("tagLine").and_then(Value::as_str).unwrap_or("")))
                .unwrap_or_else(|| format!("Player-{}", &subject[..4.min(subject.len())].to_uppercase())),
            "rankTier": tier,
            "rank": rank.name,
            "rankColor": rank.color,
            "agent": agent_name,
            "agentPortrait": catalog::resolve_agent(agent_name).map(|a| a.portrait).unwrap_or_default(),
            "agentColor": catalog::resolve_agent(agent_name).map(|a| a.color).unwrap_or("#8B978F"),
            "level": player.get("accountLevel").and_then(Value::as_i64).unwrap_or(0),
            "kills": pstats.get("kills").and_then(Value::as_i64).unwrap_or(0),
            "deaths": pstats.get("deaths").and_then(Value::as_i64).unwrap_or(0),
            "assists": pstats.get("assists").and_then(Value::as_i64).unwrap_or(0),
            "acs": if rounds > 0 { pstats.get("score").and_then(Value::as_i64).unwrap_or(0) / rounds } else { 0 },
            "shotsHit": headshot_totals(detail, subject).0,
            "headshots": headshot_totals(detail, subject).1
        });
        if player_team == team_id {
            teammates.push(row);
        } else {
            opponents.push(row);
        }
    }
    let map = catalog::map_name_from_id(
        info.get("mapId").and_then(Value::as_str).unwrap_or(""),
    );
    let queue = info
        .get("queueID")
        .or_else(|| info.get("queueId"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let party_id = me.get("partyId").and_then(Value::as_str).unwrap_or("");
    let party_size = if party_id.is_empty() {
        1
    } else {
        players
            .iter()
            .filter(|player| player.get("partyId").and_then(Value::as_str) == Some(party_id))
            .count()
    };
    Some(json!({
        "matchId": mid,
        "map": map,
        "mapSplash": Value::Null,
        "mode": catalog::mode_label(queue),
        "startMillis": info.get("gameStartMillis").and_then(Value::as_i64).unwrap_or(0),
        "result": match won {
            Some(true) => "Victory",
            Some(false) => "Defeat",
            None => "Draw"
        },
        "team": team_id,
        "score": my_team.get("roundsWon").and_then(Value::as_i64).unwrap_or(0),
        "opponentScore": opponent_score,
        "agent": agent,
        "agentPortrait": catalog::resolve_agent(agent).map(|a| a.portrait).unwrap_or_default(),
        "agentColor": catalog::resolve_agent(agent).map(|a| a.color).unwrap_or("#8B978F"),
        "kills": kills,
        "deaths": deaths,
        "assists": stats.get("assists").and_then(Value::as_i64).unwrap_or(0),
        "kd": if deaths > 0 { (kills as f64 / deaths as f64 * 100.0).round() / 100.0 } else { kills as f64 },
        "acs": if rounds > 0 { stats.get("score").and_then(Value::as_i64).unwrap_or(0) / rounds } else { 0 },
        "hsPct": if hits > 0 { json!((heads as f64 / hits as f64 * 100.0).round()) } else { Value::Null },
        "partySize": party_size,
        "scores": teams.iter().filter_map(|team| Some((team.get("teamId")?.as_str()?.to_string(), team.get("roundsWon")?.clone()))).collect::<serde_json::Map<String, Value>>(),
        "teammates": teammates,
        "opponents": opponents
    }))
}

fn headshot_totals(detail: &Value, puuid: &str) -> (i64, i64) {
    let mut hits = 0;
    let mut heads = 0;
    if let Some(rounds) = detail.get("roundResults").and_then(Value::as_array) {
        for round in rounds {
            if let Some(stats) = round.get("playerStats").and_then(Value::as_array) {
                for player in stats {
                    if player.get("subject").and_then(Value::as_str) != Some(puuid) {
                        continue;
                    }
                    if let Some(damage) = player.get("damage").and_then(Value::as_array) {
                        for hit in damage {
                            hits += hit.get("legshots").and_then(Value::as_i64).unwrap_or(0)
                                + hit.get("bodyshots").and_then(Value::as_i64).unwrap_or(0)
                                + hit.get("headshots").and_then(Value::as_i64).unwrap_or(0);
                            heads += hit.get("headshots").and_then(Value::as_i64).unwrap_or(0);
                        }
                    }
                }
            }
        }
    }
    (hits, heads)
}

fn career_summary(matches: &[Value]) -> Value {
    let n = matches.len();
    if n == 0 {
        return json!({
            "averages": { "games": 0, "wins": 0, "winRate": 0, "kd": 0, "kills": 0, "deaths": 0, "assists": 0, "hsPct": 0 },
            "coPlayers": [],
            "agentPool": [],
            "mapStats": []
        });
    }
    let wins = matches
        .iter()
        .filter(|row| row.get("result").and_then(Value::as_str) == Some("Victory"))
        .count();
    let kills: i64 = matches
        .iter()
        .map(|row| row.get("kills").and_then(Value::as_i64).unwrap_or(0))
        .sum();
    let deaths: i64 = matches
        .iter()
        .map(|row| row.get("deaths").and_then(Value::as_i64).unwrap_or(0))
        .sum();
    let assists: i64 = matches
        .iter()
        .map(|row| row.get("assists").and_then(Value::as_i64).unwrap_or(0))
        .sum();
    let hs: Vec<f64> = matches
        .iter()
        .filter_map(|row| row.get("hsPct").and_then(Value::as_f64))
        .collect();
    let mut seen: HashMap<String, (String, usize, Vec<String>)> = HashMap::new();
    for row in matches {
        if let Some(mates) = row.get("teammates").and_then(Value::as_array) {
            for mate in mates {
                let Some(puuid) = mate.get("puuid").and_then(Value::as_str) else {
                    continue;
                };
                let entry = seen.entry(puuid.to_string()).or_insert_with(|| {
                    (
                        mate.get("name").and_then(Value::as_str).unwrap_or("Unknown").to_string(),
                        0,
                        Vec::new(),
                    )
                });
                entry.1 += 1;
                if let Some(agent) = mate.get("agent").and_then(Value::as_str) {
                    if !entry.2.iter().any(|existing| existing == agent) {
                        entry.2.push(agent.to_string());
                    }
                }
            }
        }
    }
    let mut co_players: Vec<Value> = seen
        .into_iter()
        .map(|(puuid, (name, shared, agents))| {
            json!({
                "puuid": puuid,
                "name": name,
                "sharedMatches": shared,
                "agents": agents,
                "isParty": shared >= 2
            })
        })
        .collect();
    co_players.sort_by_key(|row| -row.get("sharedMatches").and_then(Value::as_i64).unwrap_or(0));
    co_players.truncate(6);
    let mut agent_pool = tally(matches, "agent");
    agent_pool.truncate(5);
    let map_stats = tally(matches, "map");
    json!({
        "averages": {
            "games": n,
            "wins": wins,
            "winRate": (100.0 * wins as f64 / n as f64).round(),
            "kills": (kills as f64 / n as f64 * 10.0).round() / 10.0,
            "deaths": (deaths as f64 / n as f64 * 10.0).round() / 10.0,
            "assists": (assists as f64 / n as f64 * 10.0).round() / 10.0,
            "kd": if deaths > 0 { (kills as f64 / deaths as f64 * 100.0).round() / 100.0 } else { kills as f64 },
            "hsPct": if hs.is_empty() { Value::Null } else { json!((hs.iter().sum::<f64>() / hs.len() as f64).round()) }
        },
        "coPlayers": co_players,
        "agentPool": agent_pool,
        "mapStats": map_stats
    })
}

fn tally(matches: &[Value], key: &str) -> Vec<Value> {
    let mut totals: HashMap<String, (i64, i64)> = HashMap::new();
    for row in matches {
        let Some(name) = row.get(key).and_then(Value::as_str) else {
            continue;
        };
        if name.is_empty() || name == "Unknown" {
            continue;
        }
        let entry = totals.entry(name.to_string()).or_default();
        entry.1 += 1;
        if row.get("result").and_then(Value::as_str) == Some("Victory") {
            entry.0 += 1;
        }
    }
    let mut rows: Vec<Value> = totals
        .into_iter()
        .map(|(name, (wins, games))| {
            json!({
                "agent": name,
                "map": name,
                "games": games,
                "wins": wins,
                "winRate": (100.0 * wins as f64 / games as f64).round()
            })
        })
        .collect();
    rows.sort_by_key(|row| -row.get("games").and_then(Value::as_i64).unwrap_or(0));
    rows
}

pub async fn match_detail(
    state: &Arc<AppState>,
    match_id: &str,
    subject: Option<&str>,
) -> Result<Value, String> {
    if let Some((value, at)) = state.details.lock().get(match_id) {
        if at.elapsed().as_secs() < 600 {
            return Ok(value.clone());
        }
    }
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let detail = session
        .match_detail(match_id)
        .await
        .map_err(|error| error.to_string())?;
    if detail.get("players").is_none() {
        return Err("Match details are unavailable.".into());
    }
    let mut value = parse_match_detail(&detail, match_id, subject);
    let subjects: Vec<String> = detail
        .get("players")
        .and_then(Value::as_array)
        .map(|players| {
            players
                .iter()
                .filter_map(|player| player.get("subject").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let names = session.reveal_names(&subjects).await;
    if let Some(players) = value.get_mut("players").and_then(Value::as_array_mut) {
        for player in players {
            if let Some(name) = player
                .get("puuid")
                .and_then(Value::as_str)
                .and_then(|puuid| names.get(puuid))
            {
                player["name"] = json!(name);
            }
        }
    }
    let map = value.get("map").and_then(Value::as_str).unwrap_or("");
    value["mapSplash"] = json!(state.valo.map_splash(map).await);
    state
        .details
        .lock()
        .insert(match_id.to_string(), (value.clone(), Instant::now()));
    Ok(value)
}

fn parse_match_detail(detail: &Value, match_id: &str, subject: Option<&str>) -> Value {
    let info = detail.get("matchInfo").cloned().unwrap_or_else(|| json!({}));
    let teams = detail
        .get("teams")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rounds = teams
        .iter()
        .map(|team| team.get("roundsWon").and_then(Value::as_i64).unwrap_or(0))
        .sum::<i64>()
        .max(1);
    let queue = info
        .get("queueID")
        .or_else(|| info.get("queueId"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    let round_mode = !matches!(queue.as_str(), "deathmatch" | "hurm");
    let raw_players = detail
        .get("players")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut hits: HashMap<String, i64> = HashMap::new();
    let mut heads: HashMap<String, i64> = HashMap::new();
    let mut dealt: HashMap<String, i64> = HashMap::new();
    let mut first_kills: HashMap<String, i64> = HashMap::new();
    let mut first_deaths: HashMap<String, i64> = HashMap::new();
    let mut multis: HashMap<String, i64> = HashMap::new();
    let mut aces: HashMap<String, i64> = HashMap::new();
    let mut plants: HashMap<String, i64> = HashMap::new();
    let mut defuses: HashMap<String, i64> = HashMap::new();
    let subject_team = raw_players
        .iter()
        .find(|player| player.get("subject").and_then(Value::as_str) == subject)
        .and_then(|player| player.get("teamId"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut timeline = Vec::new();
    for round in detail
        .get("roundResults")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let mut round_kills = Vec::new();
        for stats in round
            .get("playerStats")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let player = stats
                .get("subject")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if let Some(damage) = stats.get("damage").and_then(Value::as_array) {
                for hit in damage {
                    *hits.entry(player.clone()).or_default() += hit
                        .get("legshots")
                        .and_then(Value::as_i64)
                        .unwrap_or(0)
                        + hit.get("bodyshots").and_then(Value::as_i64).unwrap_or(0)
                        + hit.get("headshots").and_then(Value::as_i64).unwrap_or(0);
                    *heads.entry(player.clone()).or_default() +=
                        hit.get("headshots").and_then(Value::as_i64).unwrap_or(0);
                    *dealt.entry(player.clone()).or_default() +=
                        hit.get("damage").and_then(Value::as_i64).unwrap_or(0);
                }
            }
            let kills = stats
                .get("kills")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if kills.len() >= 3 {
                *multis.entry(player.clone()).or_default() += 1;
            }
            if kills.len() >= 5 {
                *aces.entry(player.clone()).or_default() += 1;
            }
            round_kills.extend(kills);
        }
        if let Some(first) = round_kills
            .iter()
            .min_by_key(|kill| kill.get("roundTime").and_then(Value::as_i64).unwrap_or(0))
        {
            if let Some(killer) = first.get("killer").and_then(Value::as_str) {
                *first_kills.entry(killer.to_string()).or_default() += 1;
            }
            if let Some(victim) = first.get("victim").and_then(Value::as_str) {
                *first_deaths.entry(victim.to_string()).or_default() += 1;
            }
        }
        if let Some(planter) = round.get("bombPlanter").and_then(Value::as_str) {
            *plants.entry(planter.to_string()).or_default() += 1;
        }
        if let Some(defuser) = round.get("bombDefuser").and_then(Value::as_str) {
            *defuses.entry(defuser.to_string()).or_default() += 1;
        }
        if let (Some(subject), Some(team)) = (subject, subject_team.as_deref()) {
            let won = round.get("winningTeam").and_then(Value::as_str) == Some(team);
            timeline.push(json!({
                "n": round.get("roundNum").and_then(Value::as_i64).unwrap_or(0) + 1,
                "won": won,
                "how": round.get("roundResult"),
                "kills": round_kills.iter().filter(|kill| kill.get("killer").and_then(Value::as_str) == Some(subject)).count(),
                "died": round_kills.iter().any(|kill| kill.get("victim").and_then(Value::as_str) == Some(subject))
            }));
        }
    }
    let mut players: Vec<Value> = raw_players
        .iter()
        .map(|player| {
            let subject_id = player.get("subject").and_then(Value::as_str).unwrap_or("");
            let stats = player.get("stats").cloned().unwrap_or_else(|| json!({}));
            let kills = stats.get("kills").and_then(Value::as_i64).unwrap_or(0);
            let deaths = stats.get("deaths").and_then(Value::as_i64).unwrap_or(0);
            let assists = stats.get("assists").and_then(Value::as_i64).unwrap_or(0);
            let tier = player.get("competitiveTier").and_then(Value::as_i64).unwrap_or(0) as i32;
            let rank = catalog::rank_from_tier(tier);
            let agent = player
                .get("characterId")
                .and_then(Value::as_str)
                .and_then(catalog::agent_name_by_uuid)
                .unwrap_or("Unknown");
            let agent_meta = catalog::resolve_agent(agent);
            let hit_count = *hits.get(subject_id).unwrap_or(&0);
            json!({
                "puuid": subject_id,
                "name": player.get("gameName").and_then(Value::as_str).filter(|name| !name.is_empty())
                    .map(|name| format!("{name}#{}", player.get("tagLine").and_then(Value::as_str).unwrap_or("")))
                    .unwrap_or_else(|| format!("Player-{}", &subject_id[..4.min(subject_id.len())].to_uppercase())),
                "team": player.get("teamId"),
                "agent": agent,
                "agentPortrait": agent_meta.as_ref().map(|a| a.portrait.clone()),
                "agentColor": agent_meta.as_ref().map(|a| a.color).unwrap_or("#8B978F"),
                "kills": kills,
                "deaths": deaths,
                "assists": assists,
                "kd": if deaths > 0 { (kills as f64 / deaths as f64 * 100.0).round() / 100.0 } else { kills as f64 },
                "acs": stats.get("score").and_then(Value::as_i64).unwrap_or(0) / rounds,
                "hsPct": if hit_count > 0 { json!(((*heads.get(subject_id).unwrap_or(&0) as f64 / hit_count as f64) * 100.0).round()) } else { Value::Null },
                "adr": if round_mode { json!((*dealt.get(subject_id).unwrap_or(&0) / rounds)) } else { Value::Null },
                "firstBloods": if round_mode { json!(*first_kills.get(subject_id).unwrap_or(&0)) } else { Value::Null },
                "firstDeaths": if round_mode { json!(*first_deaths.get(subject_id).unwrap_or(&0)) } else { Value::Null },
                "multiKills": if round_mode { json!(*multis.get(subject_id).unwrap_or(&0)) } else { Value::Null },
                "aces": if round_mode { json!(*aces.get(subject_id).unwrap_or(&0)) } else { Value::Null },
                "plants": if round_mode { json!(*plants.get(subject_id).unwrap_or(&0)) } else { Value::Null },
                "defuses": if round_mode { json!(*defuses.get(subject_id).unwrap_or(&0)) } else { Value::Null },
                "rankTier": tier,
                "rank": rank.name,
                "rankColor": rank.color,
                "rankIcon": if tier > 0 { json!(catalog::rank_icon(tier)) } else { Value::Null },
                "rr": 0,
                "leaderboard": 0,
                "peakRankTier": 0,
                "peakRank": "Unranked",
                "peakColor": "#4A4A4A",
                "peakIcon": Value::Null,
                "level": player.get("accountLevel").and_then(Value::as_i64).unwrap_or(0),
                "playerCard": Value::Null,
                "isSubject": subject == Some(subject_id)
            })
        })
        .collect();
    players.sort_by_key(|player| -player.get("acs").and_then(Value::as_i64).unwrap_or(0));
    if let Some(first) = players.first_mut() {
        first["isMatchMvp"] = json!(true);
    }
    if let Some(team) = subject_team.as_deref() {
        if let Some(first) = players
            .iter_mut()
            .find(|player| player.get("team").and_then(Value::as_str) == Some(team))
        {
            first["isTeamMvp"] = json!(true);
        }
    }
    let won = subject.and_then(|subject| {
        let player = raw_players
            .iter()
            .find(|player| player.get("subject").and_then(Value::as_str) == Some(subject))?;
        let team = player.get("teamId").and_then(Value::as_str)?;
        teams
            .iter()
            .find(|team_info| team_info.get("teamId").and_then(Value::as_str) == Some(team))
            .and_then(|team_info| team_info.get("won").and_then(Value::as_bool))
    });
    let mut team_stats = serde_json::Map::new();
    for team in &teams {
        let team_id = team.get("teamId").and_then(Value::as_str).unwrap_or("");
        let members: Vec<&Value> = players
            .iter()
            .filter(|player| player.get("team").and_then(Value::as_str) == Some(team_id))
            .collect();
        let ranked: Vec<i64> = members
            .iter()
            .filter_map(|player| player.get("rankTier").and_then(Value::as_i64))
            .filter(|tier| *tier > 0)
            .collect();
        let average = if ranked.is_empty() {
            0
        } else {
            (ranked.iter().sum::<i64>() as f64 / ranked.len() as f64).round() as i32
        };
        let rank = catalog::rank_from_tier(average);
        team_stats.insert(
            team_id.to_string(),
            json!({
                "avgRankTier": average,
                "avgRank": rank.name,
                "avgRankColor": rank.color,
                "rankIcon": if average > 0 { json!(catalog::rank_icon(average)) } else { Value::Null }
            }),
        );
    }
    let map = catalog::map_name_from_id(
        info.get("mapId").and_then(Value::as_str).unwrap_or(""),
    );
    json!({
        "matchId": match_id,
        "map": map,
        "mapSplash": Value::Null,
        "mode": catalog::mode_label(&queue),
        "scores": teams.iter().filter_map(|team| Some((team.get("teamId")?.as_str()?.to_string(), team.get("roundsWon")?.clone()))).collect::<serde_json::Map<String, Value>>(),
        "result": match won {
            Some(true) => "Victory",
            Some(false) => "Defeat",
            None => if subject.is_some() { "Draw" } else { "Unknown" }
        },
        "players": players,
        "teamStats": team_stats,
        "rounds": if round_mode { timeline } else { Vec::new() }
    })
}
