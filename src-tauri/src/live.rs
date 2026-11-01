use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use futures::{stream, StreamExt};
use serde_json::{json, Value};

use crate::{
    app::AppState,
    catalog::{self, PARTY_COLORS},
    riot::{RankInfo, Session},
};

#[derive(Clone, Default)]
pub struct PlayerStats {
    pub kd: Option<f64>,
    pub hs: Option<f64>,
    pub rr_earned: Option<i64>,
    pub intel: Option<Value>,
    pub level: i32,
    pub party_mates: Vec<String>,
    pub done: bool,
    pub fetching: bool,
    pub at: Option<Instant>,
}

impl PlayerStats {
    fn fresh(&self) -> bool {
        self.at
            .map(|at| at.elapsed() < Duration::from_secs(300))
            .unwrap_or(false)
    }
}

pub async fn board(state: &Arc<AppState>) -> Result<Value, String> {
    let refresh = state.settings_snapshot().live_refresh_secs.max(1);
    if let Some(cached) = state.live_cache.lock().as_ref() {
        if cached.at.elapsed() < Duration::from_secs(refresh) {
            return Ok(cached.value.clone());
        }
    }
    let settings = state.settings_snapshot();
    let session = match state.riot.session(&settings.region).await {
        Ok(session) => session,
        Err(error) => {
            if let Some(cached) = state.live_cache.lock().as_ref() {
                if cached.at.elapsed() < Duration::from_secs(60) {
                    return Ok(cached.value.clone());
                }
            }
            return Ok(json!({
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
                    "message": error.to_string()
                }
            }));
        }
    };
    let value = match build(state, session).await {
        Ok(value) => value,
        Err(error) => {
            if let Some(cached) = state.live_cache.lock().as_ref() {
                if cached.at.elapsed() < Duration::from_secs(90) {
                    return Ok(cached.value.clone());
                }
            }
            return Ok(json!({
                "state": "CLIENT_BUSY",
                "stateLabel": "Client busy",
                "source": "local",
                "players": [],
                "teams": {},
                "parties": [],
                "notice": {
                    "level": "warn",
                    "action": "retry",
                    "message": "Riot's local service is still starting. Retrying automatically."
                },
                "detail": error
            }));
        }
    };
    if let Some(self_player) = value
        .get("players")
        .and_then(Value::as_array)
        .and_then(|players| {
            players
                .iter()
                .find(|player| player.get("isSelf").and_then(Value::as_bool) == Some(true))
        })
    {
        if let (Some(puuid), Some(name)) = (
            self_player.get("puuid").and_then(Value::as_str),
            self_player.get("name").and_then(Value::as_str),
        ) {
            crate::history::remember_account(state, puuid, Some(name));
        }
    }
    crate::encounters::record_board(state, &value);
    *state.live_cache.lock() = Some(crate::app::CachedLive {
        at: Instant::now(),
        value: value.clone(),
    });
    Ok(value)
}

async fn build(state: &Arc<AppState>, session: Arc<Session>) -> Result<Value, String> {
    let presences = session.presences().await.unwrap_or_default();
    let mut game_state = session.game_state(&presences);
    if game_state == "OFFLINE" {
        game_state = session
            .probe_state()
            .await
            .map_err(|error| error.to_string())?;
    }
    if game_state == "MENUS" {
        return build_lobby(state, session.clone(), &presences).await;
    }
    if game_state != "PREGAME" && game_state != "INGAME" {
        return Ok(json!({
            "state": game_state,
            "stateLabel": if game_state == "OFFLINE" { "Offline" } else { &game_state },
            "source": "local",
            "players": [],
            "teams": {},
            "parties": [],
            "notice": {
                "level": "warn",
                "action": "restart_game",
                "message": "Open VALORANT to see live ranks, parties and stats."
            }
        }));
    }

    let Some(current) = session
        .current_match(&game_state)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(json!({
            "state": "MENUS",
            "stateLabel": "In Lobby",
            "source": "local",
            "players": [],
            "teams": {},
            "parties": []
        }));
    };

    let seasons = season_data(state, &session).await;
    let name_targets: Vec<String> = current
        .players
        .iter()
        .filter_map(|player| player.get("Subject").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    let (names, titles, rank_icons) = tokio::join!(
        session.reveal_names(&name_targets),
        state.valo.titles(),
        state.valo.rank_icons()
    );

    let raw_loadouts = session
        .loadouts(&game_state, &current.match_id)
        .await
        .unwrap_or_default();
    let loadout_rows = stream::iter(raw_loadouts.into_iter())
        .map(|(puuid, items)| {
            let valo = state.valo.clone();
            async move {
                let rows = valo.loadout_weapons(&items).await;
                (puuid, rows)
            }
        })
        .buffer_unordered(10)
        .collect::<HashMap<String, Vec<Value>>>()
        .await;

    let own_party = session.own_party_subjects(&presences).await;
    let stats = state.stats.lock().clone();
    let map_name = catalog::map_name_from_id(&current.map_id);
    let map_splash = state.valo.map_splash(&map_name).await;

    let mut prepared = Vec::new();
    for raw in &current.players {
        let puuid = raw
            .get("Subject")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if puuid.is_empty() {
            continue;
        }
        let identity = raw
            .get("PlayerIdentity")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let agent_id = raw
            .get("CharacterID")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let team = raw
            .get("TeamID")
            .and_then(Value::as_str)
            .unwrap_or("Blue")
            .to_string();
        let stats_entry = stats.get(&puuid).cloned().unwrap_or_default();
        let level = identity
            .get("AccountLevel")
            .and_then(Value::as_i64)
            .unwrap_or(stats_entry.level as i64) as i32;
        let title_id = identity
            .get("PlayerTitleID")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let card_id = identity
            .get("PlayerCardID")
            .and_then(Value::as_str)
            .unwrap_or("");
        let name = names.get(&puuid).cloned().unwrap_or_else(|| {
            if identity.get("Incognito").and_then(Value::as_bool) == Some(true) {
                format!("Player-{}", puuid.chars().take(4).collect::<String>().to_uppercase())
            } else {
                format!("Player-{}", puuid.chars().take(4).collect::<String>().to_uppercase())
            }
        });
        let weapons = loadout_rows
            .get(&puuid.to_lowercase())
            .cloned()
            .unwrap_or_default();
        let vandal = weapons
            .iter()
            .find(|row| row.get("weapon").and_then(Value::as_str) == Some("Vandal"))
            .and_then(|row| row.get("skin").cloned());
        prepared.push(PreparedPlayer {
            puuid,
            name,
            team,
            is_self: false,
            name_hidden: identity
                .get("Incognito")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            level,
            level_hidden: identity
                .get("HideAccountLevel")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            agent_id,
            selection: raw
                .get("CharacterSelectionState")
                .and_then(Value::as_str)
                .map(str::to_string),
            title: titles.get(&title_id).cloned(),
            player_card: (!card_id.is_empty())
                .then(|| format!("https://media.valorant-api.com/playercards/{card_id}/wideart.png")),
            weapons,
            vandal,
            stats: stats_entry,
        });
    }
    for player in &mut prepared {
        player.is_self = player.puuid == session.puuid;
    }
    let stats_targets: Vec<String> = prepared.iter().map(|player| player.puuid.clone()).collect();

    let ranks = fetch_ranks(state, &session, &prepared, &seasons).await;
    let players = build_players(
        state,
        prepared,
        &ranks,
        &rank_icons,
        &own_party,
        &presences,
        &game_state,
        &map_name,
    )
    .await;
    let self_team = players
        .iter()
        .find(|player| player.get("isSelf").and_then(Value::as_bool) == Some(true))
        .and_then(|player| player.get("team").and_then(Value::as_str))
        .unwrap_or("Blue")
        .to_string();
    let parties = party_outputs(&players);
    let score = session
        .match_score(&presences)
        .filter(|_| game_state == "INGAME");
    let locked = players
        .iter()
        .filter(|player| player.get("selection").and_then(Value::as_str) == Some("locked"))
        .count();
    let board = finalize(
        players,
        &game_state,
        &self_team,
        &map_name,
        &current.queue,
        &current.match_id,
        map_splash,
        parties,
        score,
        Some(json!({ "locked": locked, "total": current.players.len() })),
    );
    spawn_stats(state, session.clone(), stats_targets);
    Ok(board)
}

struct PreparedPlayer {
    puuid: String,
    name: String,
    team: String,
    is_self: bool,
    name_hidden: bool,
    level: i32,
    level_hidden: bool,
    agent_id: String,
    selection: Option<String>,
    title: Option<String>,
    player_card: Option<String>,
    weapons: Vec<Value>,
    vandal: Option<Value>,
    stats: PlayerStats,
}

struct SeasonData {
    active: Option<String>,
    previous: Option<String>,
}

async fn season_data(state: &Arc<AppState>, session: &Session) -> SeasonData {
    if let Some(cached) = state.meta.lock().get("seasons") {
        if let (Some(active), Some(previous)) = (
            cached.get("active").and_then(Value::as_str),
            cached.get("previous").and_then(Value::as_str),
        ) {
            return SeasonData {
                active: Some(active.to_string()),
                previous: Some(previous.to_string()),
            };
        }
    }
    let seasons = session.seasons().await;
    let active = Session::active_season(&seasons);
    let previous = Session::previous_season(&seasons);
    state.meta.lock().insert(
        "seasons".into(),
        json!({ "active": active, "previous": previous }),
    );
    SeasonData { active, previous }
}

async fn fetch_ranks(
    state: &Arc<AppState>,
    session: &Session,
    players: &[PreparedPlayer],
    seasons: &SeasonData,
) -> HashMap<String, RankInfo> {
    let mut cached = HashMap::new();
    let mut missing = Vec::new();
    {
        let cache = state.ranks.lock();
        for player in players {
            if let Some((rank, at)) = cache.get(&player.puuid) {
                if at.elapsed() < Duration::from_secs(60) {
                    cached.insert(player.puuid.clone(), rank.clone());
                    continue;
                }
            }
            missing.push(player.puuid.clone());
        }
    }
    if !missing.is_empty() {
        let fetched = stream::iter(missing)
            .map(|puuid| async move {
                let rank = session
                    .rank_info(&puuid, seasons.active.as_deref(), seasons.previous.as_deref())
                    .await;
                (puuid, rank)
            })
            .buffer_unordered(6)
            .collect::<Vec<_>>()
            .await;
        let mut cache = state.ranks.lock();
        for (puuid, rank) in fetched {
            if rank.ok {
                cache.insert(puuid.clone(), (rank.clone(), Instant::now()));
            }
            cached.insert(puuid, rank);
        }
    }
    cached
}

#[allow(clippy::too_many_arguments)]
async fn build_players(
    state: &Arc<AppState>,
    prepared: Vec<PreparedPlayer>,
    ranks: &HashMap<String, RankInfo>,
    rank_icons: &HashMap<i32, (Option<String>, Option<String>)>,
    own_party: &[String],
    presences: &[Value],
    game_state: &str,
    map_name: &str,
) -> Vec<Value> {
    let mut party_sets: HashMap<String, HashSet<String>> = HashMap::new();
    for puuid in own_party {
        party_sets
            .entry("own".into())
            .or_default()
            .insert(puuid.clone());
    }
    for presence in presences {
        let Some(private) = presence
            .get("private")
            .and_then(Value::as_str)
            .and_then(crate::riot::decode_private)
        else {
            continue;
        };
        let data = private.get("partyPresenceData").unwrap_or(&private);
        let party_id = data.get("partyId").and_then(Value::as_str);
        let size = data.get("partySize").and_then(Value::as_u64).unwrap_or(1);
        if let (Some(party_id), true) = (party_id, size > 1) {
            if let Some(puuid) = presence.get("puuid").and_then(Value::as_str) {
                party_sets
                    .entry(party_id.to_string())
                    .or_default()
                    .insert(puuid.to_string());
            }
        }
    }
    for player in &prepared {
        for mate in &player.stats.party_mates {
            let pair = [player.puuid.clone(), mate.clone()];
            party_sets.entry(format!("inferred-{}", pair[0])).or_default().extend(pair);
        }
        if let Some(party_id) = player
            .stats
            .intel
            .as_ref()
            .and_then(|intel| intel.get("partyId"))
            .and_then(Value::as_str)
        {
            party_sets
                .entry(format!("match-{party_id}"))
                .or_default()
                .insert(player.puuid.clone());
        }
    }

    let present: HashSet<String> = prepared.iter().map(|p| p.puuid.clone()).collect();
    let mut valid_sets: Vec<Vec<String>> = party_sets
        .into_values()
        .map(|set| {
            set.into_iter()
                .filter(|puuid| present.contains(puuid))
                .collect::<Vec<_>>()
        })
        .filter(|set| set.len() > 1)
        .collect();
    valid_sets.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a[0].cmp(&b[0])));
    let mut lookup: HashMap<String, Value> = HashMap::new();
    let mut number = 0usize;
    for set in valid_sets {
        number += 1;
        let color = PARTY_COLORS[(number - 1) % PARTY_COLORS.len()];
        for puuid in &set {
            lookup.insert(
                puuid.clone(),
                json!({
                    "id": format!("party-{number}"),
                    "color": color,
                    "number": number,
                    "inferred": false
                }),
            );
        }
    }

    let mut players = Vec::new();
    for player in prepared {
        let agent = catalog::resolve_agent(&player.agent_id);
        let rank = ranks.get(&player.puuid).cloned().unwrap_or_default();
        let rank_meta = catalog::rank_from_tier(rank.tier);
        let peak_meta = catalog::rank_from_tier(rank.peak);
        let previous_meta = catalog::rank_from_tier(rank.previous);
        let rank_icon = (rank.tier > 0)
            .then(|| rank_icons.get(&rank.tier).and_then(|(small, _)| small.clone()))
            .flatten();
        let peak_icon = (rank.peak > 0)
            .then(|| rank_icons.get(&rank.peak).and_then(|(small, _)| small.clone()))
            .flatten();
        let (smurf, reasons) = smurf_signals(
            player.level,
            rank.peak,
            player.stats.kd,
            rank.win_rate,
            rank.games,
        );
        let peak_act = if rank.peak > 0 && !rank.peak_season.is_empty() {
            state.valo.season_label(&rank.peak_season).await
        } else {
            None
        };
        let pending = game_state != "MENUS"
            && (!player.stats.done || player.stats.kd.is_none())
            && !player.stats.fresh();
        let form = player
            .stats
            .intel
            .as_ref()
            .and_then(|intel| intel.get("form"))
            .cloned()
            .unwrap_or_else(|| json!([]));
        let streak = player
            .stats
            .intel
            .as_ref()
            .and_then(|intel| intel.get("streak"))
            .cloned()
            .unwrap_or(Value::Null);
        let top_agents = player
            .stats
            .intel
            .as_ref()
            .and_then(|intel| intel.get("topAgents"))
            .cloned()
            .unwrap_or_else(|| json!([]));
        let map_win_rate = player
            .stats
            .intel
            .as_ref()
            .and_then(|intel| intel.get("mapWins"))
            .cloned()
            .unwrap_or(Value::Null);
        players.push(json!({
            "puuid": player.puuid,
            "name": player.name,
            "nameHidden": player.name_hidden,
            "team": player.team,
            "isSelf": player.is_self,
            "title": player.title,
            "playerCard": player.player_card,
            "agent": agent.as_ref().map(|a| a.name),
            "agentId": agent.as_ref().map(|a| a.uuid),
            "agentPortrait": agent.as_ref().map(|a| a.portrait.clone()),
            "agentArt": agent.as_ref().map(|a| a.full_portrait.clone()),
            "agentColor": agent.as_ref().map(|a| a.color).unwrap_or("#8B978F"),
            "role": agent.as_ref().map(|a| a.role),
            "selection": player.selection,
            "rankTier": rank.tier,
            "rank": rank_meta.name,
            "rankColor": rank_meta.color,
            "rankGroup": rank_meta.group,
            "rankIcon": rank_icon,
            "rr": rank.rr,
            "rrEarned": player.stats.rr_earned,
            "leaderboard": rank.leaderboard,
            "peakRankTier": rank.peak,
            "peakRank": peak_meta.name,
            "peakColor": peak_meta.color,
            "peakIcon": peak_icon,
            "peakAct": peak_act,
            "previousRank": previous_meta.name,
            "winRate": rank.win_rate,
            "games": rank.games,
            "kd": player.stats.kd,
            "hsPct": player.stats.hs,
            "skin": player.vandal,
            "weapons": player.weapons,
            "level": player.level,
            "levelHidden": player.level_hidden,
            "party": lookup.get(&player.puuid),
            "smurf": smurf,
            "smurfReasons": reasons,
            "topAgents": top_agents,
            "form": form,
            "streak": streak,
            "mapWinRate": current_map_rate(&map_win_rate, map_name),
            "statsPending": pending
        }));
    }
    players
}

fn current_map_rate(map_wins: &Value, map_name: &str) -> Value {
    let Some(row) = map_wins.get(map_name).and_then(Value::as_array) else {
        return Value::Null;
    };
    let wins = row.first().and_then(Value::as_i64).unwrap_or(0);
    let games = row.get(1).and_then(Value::as_i64).unwrap_or(0);
    if games <= 0 {
        return Value::Null;
    }
    json!({
        "winRate": ((wins as f64 / games as f64) * 100.0).round(),
        "games": games
    })
}

fn smurf_signals(
    level: i32,
    peak_tier: i32,
    kd: Option<f64>,
    win_rate: i32,
    games: i32,
) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();
    if level <= 0 {
        return (false, reasons);
    }
    if level < 60 && peak_tier >= 20 {
        reasons.push(format!(
            "Lvl {level}, peak {}",
            catalog::rank_from_tier(peak_tier).name
        ));
    }
    if let Some(kd) = kd {
        if kd >= 1.35 && level < 80 {
            reasons.push(format!("K/D {kd:.2} at lvl {level}"));
        }
    }
    if win_rate >= 62 && games >= 15 && level < 100 {
        reasons.push(format!("{win_rate}% WR"));
    }
    let flagged = (level < 60 && !reasons.is_empty()) || reasons.len() >= 2;
    (flagged, reasons)
}

fn party_outputs(players: &[Value]) -> Vec<Value> {
    let mut parties: HashMap<String, Value> = HashMap::new();
    for player in players {
        let Some(party) = player.get("party") else {
            continue;
        };
        let Some(id) = party.get("id").and_then(Value::as_str) else {
            continue;
        };
        let entry = parties.entry(id.to_string()).or_insert_with(|| {
            json!({
                "id": id,
                "color": party.get("color"),
                "number": party.get("number"),
                "size": 0,
                "members": [],
                "inferred": party.get("inferred").unwrap_or(&json!(false))
            })
        });
        if let Some(size) = entry.get("size").and_then(Value::as_u64) {
            entry["size"] = json!(size + 1);
        }
        if let Some(members) = entry.get_mut("members").and_then(Value::as_array_mut) {
            members.push(json!(player.get("puuid")));
        }
    }
    let mut rows: Vec<Value> = parties.into_values().collect();
    rows.sort_by_key(|party| party.get("number").and_then(Value::as_u64).unwrap_or(0));
    rows
}

#[allow(clippy::too_many_arguments)]
fn finalize(
    players: Vec<Value>,
    state: &str,
    self_team: &str,
    map: &str,
    queue: &str,
    match_id: &str,
    map_splash: Option<String>,
    parties: Vec<Value>,
    score: Option<Value>,
    lock_progress: Option<Value>,
) -> Value {
    let mut sorted = players;
    sorted.sort_by_key(|player| {
        let team = player.get("team").and_then(Value::as_str).unwrap_or("");
        let rank = player.get("rankTier").and_then(Value::as_i64).unwrap_or(0);
        let level = player.get("level").and_then(Value::as_i64).unwrap_or(0);
        (team != self_team, -(rank), -(level))
    });
    let mut teams: HashMap<String, Vec<Value>> = HashMap::new();
    for player in &sorted {
        let team = player
            .get("team")
            .and_then(Value::as_str)
            .unwrap_or("Blue")
            .to_string();
        teams.entry(team).or_default().push(player.clone());
    }
    let mut team_stats = serde_json::Map::new();
    for (team, members) in &teams {
        team_stats.insert(team.clone(), team_stats_for(members));
    }
    let win_prob = if state == "INGAME" && teams.len() == 2 {
        let enemy = teams
            .keys()
            .find(|team| team.as_str() != self_team)
            .cloned()
            .unwrap_or_default();
        let mine = team_stats.get(self_team).cloned().unwrap_or(Value::Null);
        let theirs = team_stats.get(&enemy).cloned().unwrap_or(Value::Null);
        win_probability(&mine, &theirs)
    } else {
        None
    };
    let side = if state == "INGAME" || state == "PREGAME" {
        match self_team {
            "Red" => Some("Attacker"),
            "Blue" => Some("Defender"),
            _ => None,
        }
    } else {
        None
    };
    let mode = if state == "MENUS" {
        "Lobby"
    } else {
        catalog::mode_label(queue)
    };
    json!({
        "state": state,
        "stateLabel": match state {
            "MENUS" => "In Lobby",
            "PREGAME" => "Agent Select",
            "INGAME" => "In Game",
            "OFFLINE" => "Offline",
            other => other
        },
        "source": "local",
        "map": if map.is_empty() { Value::Null } else { json!(map) },
        "mapSplash": map_splash,
        "mode": mode,
        "matchId": match_id,
        "selfTeam": self_team,
        "side": side,
        "players": sorted,
        "teams": teams,
        "teamStats": team_stats,
        "winProb": win_prob,
        "parties": parties,
        "score": score,
        "lockProgress": lock_progress
    })
}

fn team_stats_for(players: &[Value]) -> Value {
    let tiers: Vec<i64> = players
        .iter()
        .filter_map(|player| player.get("rankTier").and_then(Value::as_i64))
        .filter(|tier| *tier > 0)
        .collect();
    let kds: Vec<f64> = players
        .iter()
        .filter_map(|player| player.get("kd").and_then(Value::as_f64))
        .collect();
    let wrs: Vec<i64> = players
        .iter()
        .filter_map(|player| player.get("winRate").and_then(Value::as_i64))
        .filter(|wr| *wr > 0)
        .collect();
    let avg_tier = if tiers.is_empty() {
        0.0
    } else {
        tiers.iter().sum::<i64>() as f64 / tiers.len() as f64
    };
    let rank = catalog::rank_from_tier(avg_tier.round() as i32);
    json!({
        "avgRankTier": (avg_tier * 100.0).round() / 100.0,
        "avgRank": rank.name,
        "rankColor": rank.color,
        "rankIcon": if tiers.is_empty() { Value::Null } else { json!(catalog::rank_icon(avg_tier.round() as i32)) },
        "avgKd": if kds.is_empty() { Value::Null } else { json!((kds.iter().sum::<f64>() / kds.len() as f64 * 100.0).round() / 100.0) },
        "avgWinRate": if wrs.is_empty() { Value::Null } else { json!(wrs.iter().sum::<i64>() / wrs.len() as i64) },
        "smurfCount": players.iter().filter(|player| player.get("smurf").and_then(Value::as_bool) == Some(true)).count(),
        "size": players.len()
    })
}

fn win_probability(mine: &Value, theirs: &Value) -> Option<i64> {
    let mut probability = 50.0;
    let mut used = false;
    let mine_rank = mine.get("avgRankTier").and_then(Value::as_f64).unwrap_or(0.0);
    let their_rank = theirs
        .get("avgRankTier")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if mine_rank > 0.0 && their_rank > 0.0 {
        probability += (mine_rank - their_rank) * 5.0;
        used = true;
    }
    if let (Some(mine_kd), Some(their_kd)) = (
        mine.get("avgKd").and_then(Value::as_f64),
        theirs.get("avgKd").and_then(Value::as_f64),
    ) {
        probability += (mine_kd - their_kd) * 20.0;
        used = true;
    }
    used.then(|| probability.round().clamp(5.0, 95.0) as i64)
}

async fn build_lobby(
    state: &Arc<AppState>,
    session: Arc<Session>,
    presences: &[Value],
) -> Result<Value, String> {
    let members = session.own_party_members(presences).await;
    let puuids: Vec<String> = if members.is_empty() {
        vec![session.puuid.clone()]
    } else {
        members
            .iter()
            .filter_map(|member| member.get("Subject").and_then(Value::as_str))
            .map(str::to_string)
            .collect()
    };
    let (names, titles, rank_icons) = tokio::join!(
        session.reveal_names(&puuids),
        state.valo.titles(),
        state.valo.rank_icons()
    );
    let seasons = season_data(state, &session).await;
    let stats = state.stats.lock().clone();
    let mut prepared = Vec::new();
    for (index, puuid) in puuids.iter().enumerate() {
        let entry = stats.get(puuid).cloned().unwrap_or_default();
        let identity = members
            .get(index)
            .and_then(|member| member.get("PlayerIdentity"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        let level = identity
            .get("AccountLevel")
            .and_then(Value::as_i64)
            .unwrap_or(entry.level as i64) as i32;
        let title_id = identity
            .get("PlayerTitleID")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let card_id = identity
            .get("PlayerCardID")
            .and_then(Value::as_str)
            .unwrap_or("");
        prepared.push(PreparedPlayer {
            puuid: puuid.clone(),
            name: names
                .get(puuid)
                .cloned()
                .unwrap_or_else(|| format!("Player-{}", &puuid[..4.min(puuid.len())].to_uppercase())),
            team: "Blue".into(),
            is_self: puuid == &session.puuid,
            name_hidden: identity
                .get("Incognito")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            level,
            level_hidden: identity
                .get("HideAccountLevel")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            agent_id: String::new(),
            selection: None,
            title: titles.get(&title_id).cloned(),
            player_card: (!card_id.is_empty())
                .then(|| format!("https://media.valorant-api.com/playercards/{card_id}/wideart.png")),
            weapons: Vec::new(),
            vandal: None,
            stats: entry,
        });
    }
    let ranks = fetch_ranks(state, &session, &prepared, &seasons).await;
    let mut players = Vec::new();
    for player in prepared {
        let rank = ranks.get(&player.puuid).cloned().unwrap_or_default();
        let rank_meta = catalog::rank_from_tier(rank.tier);
        let peak_meta = catalog::rank_from_tier(rank.peak);
        let previous_meta = catalog::rank_from_tier(rank.previous);
        let rank_icon = (rank.tier > 0)
            .then(|| rank_icons.get(&rank.tier).and_then(|(small, _)| small.clone()))
            .flatten();
        let peak_icon = (rank.peak > 0)
            .then(|| rank_icons.get(&rank.peak).and_then(|(small, _)| small.clone()))
            .flatten();
        let (smurf, reasons) = smurf_signals(
            player.level,
            rank.peak,
            player.stats.kd,
            rank.win_rate,
            rank.games,
        );
        let peak_act = if rank.peak > 0 && !rank.peak_season.is_empty() {
            state.valo.season_label(&rank.peak_season).await
        } else {
            None
        };
        let party = (puuids.len() > 1).then(|| {
            json!({
                "id": "lobby",
                "color": PARTY_COLORS[0],
                "number": 1,
                "inferred": false
            })
        });
        players.push(json!({
            "puuid": player.puuid,
            "name": player.name,
            "nameHidden": player.name_hidden,
            "team": "Blue",
            "isSelf": player.is_self,
            "title": player.title,
            "playerCard": player.player_card,
            "agent": Value::Null,
            "agentId": Value::Null,
            "agentPortrait": Value::Null,
            "agentArt": Value::Null,
            "agentColor": "#8B978F",
            "role": Value::Null,
            "selection": Value::Null,
            "rankTier": rank.tier,
            "rank": rank_meta.name,
            "rankColor": rank_meta.color,
            "rankGroup": rank_meta.group,
            "rankIcon": rank_icon,
            "rr": rank.rr,
            "rrEarned": player.stats.rr_earned,
            "leaderboard": rank.leaderboard,
            "peakRankTier": rank.peak,
            "peakRank": peak_meta.name,
            "peakColor": peak_meta.color,
            "peakIcon": peak_icon,
            "peakAct": peak_act,
            "previousRank": previous_meta.name,
            "winRate": rank.win_rate,
            "games": rank.games,
            "kd": player.stats.kd,
            "hsPct": player.stats.hs,
            "skin": Value::Null,
            "weapons": [],
            "level": player.level,
            "levelHidden": player.level_hidden,
            "party": party,
            "smurf": smurf,
            "smurfReasons": reasons,
            "topAgents": player.stats.intel.as_ref().and_then(|i| i.get("topAgents")).cloned().unwrap_or_else(|| json!([])),
            "form": player.stats.intel.as_ref().and_then(|i| i.get("form")).cloned().unwrap_or_else(|| json!([])),
            "streak": player.stats.intel.as_ref().and_then(|i| i.get("streak")).cloned().unwrap_or(Value::Null),
            "mapWinRate": Value::Null,
            "statsPending": !player.stats.done && !player.stats.fresh()
        }));
    }
    let parties = if puuids.len() > 1 {
        vec![json!({
            "id": "lobby",
            "color": PARTY_COLORS[0],
            "number": 1,
            "size": puuids.len(),
            "members": puuids,
            "inferred": false
        })]
    } else {
        Vec::new()
    };
    let queue = session
        .party_snapshot()
        .await
        .unwrap_or_else(|_| json!({ "available": false }));
    let mut board = finalize(
        players,
        "MENUS",
        "Blue",
        "",
        "custom",
        "lobby",
        None,
        parties,
        None,
        None,
    );
    board["queue"] = queue;
    spawn_stats(state, session.clone(), puuids.clone());
    Ok(board)
}

fn spawn_stats(state: &Arc<AppState>, session: Arc<Session>, puuids: Vec<String>) {
    let mut targets = Vec::new();
    {
        let mut stats = state.stats.lock();
        for puuid in puuids {
            let entry = stats.entry(puuid.clone()).or_default();
            if entry.fetching || (entry.done && entry.fresh()) {
                continue;
            }
            entry.fetching = true;
            targets.push(puuid);
        }
    }
    if targets.is_empty() {
        return;
    }
    let state = state.clone();
    tauri::async_runtime::spawn(async move {
        let results = stream::iter(targets)
            .map(|puuid| {
                let session = session.clone();
                async move {
                    let snapshot = fetch_stats(&session, &puuid).await;
                    (puuid, snapshot)
                }
            })
            .buffer_unordered(4)
            .collect::<Vec<_>>()
            .await;
        let mut cache = state.stats.lock();
        for (puuid, mut snapshot) in results {
            if let Some(existing) = cache.get(&puuid) {
                snapshot.done = snapshot.done || existing.done;
                if snapshot.kd.is_none() {
                    snapshot.kd = existing.kd;
                    snapshot.hs = existing.hs;
                    snapshot.intel = existing.intel.clone();
                }
                if snapshot.level == 0 {
                    snapshot.level = existing.level;
                }
            }
            snapshot.fetching = false;
            snapshot.at = Some(Instant::now());
            cache.insert(puuid, snapshot);
        }
    });
}

async fn fetch_stats(session: &Session, puuid: &str) -> PlayerStats {
    let (mids, _) = session.fresh_match_ids(puuid).await;
    let mids: Vec<String> = mids.into_iter().take(3).collect();
    if mids.is_empty() {
        return PlayerStats {
            done: true,
            ..Default::default()
        };
    }
    let details = stream::iter(mids.iter().cloned())
        .map(|mid| {
            let session = session.clone();
            async move { session.match_detail(&mid).await.ok() }
        })
        .buffer_unordered(3)
        .collect::<Vec<_>>()
        .await;
    let mut kills = 0i64;
    let mut deaths = 0i64;
    let mut hits = 0i64;
    let mut heads = 0i64;
    let mut used = 0i64;
    let mut agent_counts: HashMap<String, i64> = HashMap::new();
    let mut form = Vec::new();
    let mut map_wins: HashMap<String, [i64; 2]> = HashMap::new();
    let mut level = 0;
    let mut party_mates = Vec::new();
    for (index, detail) in details.into_iter().enumerate() {
        let Some(detail) = detail else {
            continue;
        };
        for round in detail
            .get("roundResults")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            for stats in round
                .get("playerStats")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
            {
                if stats.get("subject").and_then(Value::as_str) != Some(puuid) {
                    continue;
                }
                for damage in stats
                    .get("damage")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                {
                    hits += damage.get("legshots").and_then(Value::as_i64).unwrap_or(0)
                        + damage.get("bodyshots").and_then(Value::as_i64).unwrap_or(0)
                        + damage.get("headshots").and_then(Value::as_i64).unwrap_or(0);
                    heads += damage.get("headshots").and_then(Value::as_i64).unwrap_or(0);
                }
            }
        }
        let players = detail
            .get("players")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let Some(me) = players
            .iter()
            .find(|player| player.get("subject").and_then(Value::as_str) == Some(puuid))
        else {
            continue;
        };
        used += 1;
        level = me
            .get("accountLevel")
            .and_then(Value::as_i64)
            .unwrap_or(level as i64) as i32;
        let stats = me.get("stats").cloned().unwrap_or_else(|| json!({}));
        kills += stats.get("kills").and_then(Value::as_i64).unwrap_or(0);
        deaths += stats.get("deaths").and_then(Value::as_i64).unwrap_or(0);
        if let Some(agent) =
            catalog::agent_name_by_uuid(me.get("characterId").and_then(Value::as_str).unwrap_or(""))
        {
            *agent_counts.entry(agent.to_string()).or_default() += 1;
        }
        if index == 0 {
            if let Some(party_id) = me.get("partyId").and_then(Value::as_str) {
                for player in &players {
                    if player.get("partyId").and_then(Value::as_str) == Some(party_id) {
                        if let Some(subject) = player.get("subject").and_then(Value::as_str) {
                            if subject != puuid {
                                party_mates.push(subject.to_string());
                            }
                        }
                    }
                }
            }
        }
        let teams = detail
            .get("teams")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let team_id = me.get("teamId").and_then(Value::as_str).unwrap_or("");
        if let Some(team) = teams
            .iter()
            .find(|team| team.get("teamId").and_then(Value::as_str) == Some(team_id))
        {
            if let Some(won) = team.get("won").and_then(Value::as_bool) {
                form.push(if won { "W" } else { "L" });
                let map = catalog::map_name_from_id(
                    detail
                        .get("matchInfo")
                        .and_then(|info| info.get("mapId"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                );
                let entry = map_wins.entry(map).or_default();
                entry[1] += 1;
                if won {
                    entry[0] += 1;
                }
            }
        }
    }
    let kd = if deaths > 0 {
        Some((kills as f64 / deaths as f64 * 100.0).round() / 100.0)
    } else if used > 0 {
        Some(kills as f64)
    } else {
        None
    };
    let hs = if hits > 0 {
        Some((heads as f64 / hits as f64 * 100.0).round())
    } else {
        None
    };
    let mut top_agents: Vec<Value> = agent_counts
        .into_iter()
        .map(|(agent, games)| json!({ "agent": agent, "games": games }))
        .collect();
    top_agents.sort_by_key(|agent| -agent.get("games").and_then(Value::as_i64).unwrap_or(0));
    top_agents.truncate(3);
    let mut streak = None;
    if let Some(first) = form.first() {
        let count = form.iter().take_while(|result| result == &first).count();
        streak = Some(json!({ "type": first, "count": count }));
    }
    let map_wins_json: serde_json::Map<String, Value> = map_wins
        .into_iter()
        .map(|(map, wins)| (map, json!([wins[0], wins[1]])))
        .collect();
    PlayerStats {
        kd,
        hs,
        rr_earned: None,
        intel: Some(json!({
            "topAgents": top_agents,
            "form": form,
            "streak": streak,
            "mapWins": map_wins_json
        })),
        level,
        party_mates,
        done: used > 0,
        fetching: false,
        at: None,
    }
}

pub async fn party_state(state: &Arc<AppState>) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    session
        .party_snapshot()
        .await
        .map_err(|error| error.to_string())
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smurf_flags_low_level_high_peak() {
        let (flagged, reasons) = smurf_signals(20, 21, None, 0, 0);
        assert!(flagged);
        assert!(!reasons.is_empty());
    }

    #[test]
    fn experienced_account_is_not_flagged() {
        let (flagged, reasons) = smurf_signals(300, 21, Some(1.4), 70, 40);
        assert!(!flagged);
        assert!(reasons.is_empty());
    }

    #[test]
    fn win_probability_moves_with_rank_and_kd() {
        let mine = json!({ "avgRankTier": 15.0, "avgKd": 1.4 });
        let theirs = json!({ "avgRankTier": 10.0, "avgKd": 0.9 });
        let probability = win_probability(&mine, &theirs).unwrap();
        assert!(probability > 50);
    }
}
