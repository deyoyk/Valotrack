use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
    Client, Method, Response,
};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::catalog;

const PLATFORM: &str = "ew0KCSJwbGF0Zm9ybVR5cGUiOiAiUEMiLA0KCSJwbGF0Zm9ybU9TIjogIldpbmRvd3MiLA0KCSJwbGF0Zm9ybU9TVmVyc2lvbiI6ICIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwNCgkicGxhdGZvcm1DaGlwc2V0IjogIlVua25vd24iDQp9";
const SHARED_GAME_CLIENT: &str = "ShooterGame/13 Windows/10.0.19043.1.256.64bit";

#[derive(Debug, thiserror::Error)]
pub enum RiotError {
    #[error("Riot Client is not running")]
    ClientUnavailable,
    #[error("could not read the Riot lockfile")]
    Lockfile,
    #[error("Riot session is not ready: {0}")]
    NotReady(String),
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Riot rate limited this request")]
    RateLimited,
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStatus {
    pub available: bool,
    pub lockfile_path: String,
    pub region: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Lockfile {
    pub name: String,
    pub pid: u32,
    pub port: u16,
    pub password: String,
    pub protocol: String,
}

#[derive(Debug, Clone)]
pub struct RegionInfo {
    pub shard: String,
    pub glz_a: String,
    pub glz_b: String,
}

impl RegionInfo {
    pub fn label(&self) -> String {
        self.shard.clone()
    }

    pub fn glz_host(&self) -> String {
        format!("glz-{}.{}.a.pvp.net", self.glz_a, self.glz_b)
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RankInfo {
    pub tier: i32,
    pub rr: i32,
    pub leaderboard: i32,
    pub peak: i32,
    pub win_rate: i32,
    pub games: i32,
    pub previous: i32,
    pub peak_season: String,
    pub ok: bool,
}

impl Default for RankInfo {
    fn default() -> Self {
        Self {
            tier: 0,
            rr: 0,
            leaderboard: 0,
            peak: 0,
            win_rate: 0,
            games: 0,
            previous: 0,
            peak_season: String::new(),
            ok: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CurrentMatch {
    pub players: Vec<Value>,
    pub match_id: String,
    pub map_id: String,
    pub queue: String,
}

#[derive(Clone)]
pub struct Session {
    pub http: Client,
    pub rate: Arc<RateLimiter>,
    pub lockfile: Lockfile,
    pub puuid: String,
    pub region: RegionInfo,
    pub headers: HeaderMap,
    pub client_version: String,
    pub created: Instant,
}

impl Session {
    pub fn pd_url(&self, path: &str) -> String {
        format!("https://pd.{}.a.pvp.net{}", self.region.shard, path)
    }

    pub fn glz_url(&self, path: &str) -> String {
        format!("https://{}{}", self.region.glz_host(), path)
    }

    pub fn shared_url(&self, path: &str) -> String {
        format!("https://shared.{}.a.pvp.net{}", self.region.shard, path)
    }

    pub fn local_url(&self, path: &str) -> String {
        format!("https://127.0.0.1:{}{}", self.lockfile.port, path)
    }

    async fn execute(
        &self,
        method: Method,
        url: String,
        headers: HeaderMap,
        body: Option<&Value>,
        throttle: bool,
    ) -> Result<Response, RiotError> {
        if throttle {
            self.rate.acquire(url.contains("/mmr/")).await;
        }
        let mut request = self.http.request(method, url).headers(headers);
        if let Some(value) = body {
            request = request.json(value);
        }
        Ok(request.send().await?)
    }

    pub async fn local_get(&self, path: &str) -> Result<Value, RiotError> {
        let mut headers = HeaderMap::new();
        let raw = format!("riot:{}", self.lockfile.password);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {}", B64.encode(raw)))
                .map_err(|e| RiotError::Other(e.to_string()))?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        let response = self
            .execute(Method::GET, self.local_url(path), headers, None, false)
            .await?;
        decode(response).await
    }

    pub async fn pd_get(&self, path: &str) -> Result<Value, RiotError> {
        let response = self
            .execute(Method::GET, self.pd_url(path), self.headers.clone(), None, true)
            .await?;
        decode(response).await
    }

    pub async fn pd_put(&self, path: &str, body: &Value) -> Result<Value, RiotError> {
        let response = self
            .execute(
                Method::PUT,
                self.pd_url(path),
                self.headers.clone(),
                Some(body),
                true,
            )
            .await?;
        decode(response).await
    }

    pub async fn glz_get(&self, path: &str) -> Result<Value, RiotError> {
        let response = self
            .execute(Method::GET, self.glz_url(path), self.headers.clone(), None, true)
            .await?;
        decode(response).await
    }

    pub async fn glz_post(&self, path: &str, body: Option<&Value>) -> Result<Response, RiotError> {
        self.execute(
            Method::POST,
            self.glz_url(path),
            self.headers.clone(),
            body,
            true,
        )
        .await
    }

    pub async fn shared_get(&self, path: &str) -> Result<Value, RiotError> {
        let response = self
            .execute(
                Method::GET,
                self.shared_url(path),
                self.headers.clone(),
                None,
                true,
            )
            .await?;
        decode(response).await
    }

    pub async fn presences(&self) -> Result<Vec<Value>, RiotError> {
        let data = self.local_get("/chat/v4/presences").await?;
        Ok(data
            .get("presences")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    pub fn game_state(&self, presences: &[Value]) -> String {
        for presence in presences {
            if presence.get("puuid").and_then(Value::as_str) != Some(self.puuid.as_str()) {
                continue;
            }
            let product = presence.get("product").and_then(Value::as_str).unwrap_or("");
            if !product.is_empty() && product != "valorant" {
                continue;
            }
            let Some(private) = presence.get("private").and_then(Value::as_str) else {
                continue;
            };
            let Some(value) = decode_private(private) else {
                continue;
            };
            if let Some(data) = value.get("matchPresenceData") {
                return data
                    .get("sessionLoopState")
                    .and_then(Value::as_str)
                    .unwrap_or("MENUS")
                    .to_string();
            }
            return value
                .get("sessionLoopState")
                .and_then(Value::as_str)
                .unwrap_or("MENUS")
                .to_string();
        }
        "OFFLINE".into()
    }

    pub fn self_private(&self, presences: &[Value]) -> Option<Value> {
        presences.iter().find_map(|presence| {
            if presence.get("puuid").and_then(Value::as_str) != Some(self.puuid.as_str()) {
                return None;
            }
            let product = presence.get("product").and_then(Value::as_str).unwrap_or("");
            if !product.is_empty() && product != "valorant" {
                return None;
            }
            decode_private(presence.get("private").and_then(Value::as_str)?)
        })
    }

    pub fn match_score(&self, presences: &[Value]) -> Option<Value> {
        let private = self.self_private(presences)?;
        let data = private.get("matchPresenceData").unwrap_or(&private);
        let ally = data
            .get("partyOwnerMatchScoreAllyTeam")
            .or_else(|| private.get("partyOwnerMatchScoreAllyTeam"))?;
        let enemy = data
            .get("partyOwnerMatchScoreEnemyTeam")
            .or_else(|| private.get("partyOwnerMatchScoreEnemyTeam"))?;
        let ally = ally.as_i64().unwrap_or(0);
        let enemy = enemy.as_i64().unwrap_or(0);
        Some(json!({ "ally": ally, "enemy": enemy, "round": ally + enemy + 1 }))
    }

    pub async fn current_match(&self, state: &str) -> Result<Option<CurrentMatch>, RiotError> {
        if state == "INGAME" {
            let player = self
                .glz_get(&format!("/core-game/v1/players/{}", self.puuid))
                .await?;
            let Some(match_id) = player.get("MatchID").and_then(Value::as_str) else {
                return Ok(None);
            };
            let game = self
                .glz_get(&format!("/core-game/v1/matches/{match_id}"))
                .await?;
            return Ok(Some(CurrentMatch {
                players: game
                    .get("Players")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
                match_id: match_id.to_string(),
                map_id: game
                    .get("MapID")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                queue: game
                    .get("MatchmakingData")
                    .and_then(|m| m.get("QueueID"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            }));
        }
        if state == "PREGAME" {
            let player = self
                .glz_get(&format!("/pregame/v1/players/{}", self.puuid))
                .await?;
            let Some(match_id) = player.get("MatchID").and_then(Value::as_str) else {
                return Ok(None);
            };
            let game = self
                .glz_get(&format!("/pregame/v1/matches/{match_id}"))
                .await?;
            let ally = game.get("AllyTeam").cloned().unwrap_or_else(|| json!({}));
            let team_id = ally
                .get("TeamID")
                .and_then(Value::as_str)
                .unwrap_or("Blue");
            let mut players = ally
                .get("Players")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for player in &mut players {
                if let Value::Object(map) = player {
                    map.insert("TeamID".into(), json!(team_id));
                }
            }
            return Ok(Some(CurrentMatch {
                players,
                match_id: match_id.to_string(),
                map_id: game
                    .get("MapID")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                queue: game
                    .get("QueueID")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            }));
        }
        Ok(None)
    }

    pub async fn loadouts(
        &self,
        state: &str,
        match_id: &str,
    ) -> Result<HashMap<String, serde_json::Map<String, Value>>, RiotError> {
        let path = if state == "INGAME" {
            format!("/core-game/v1/matches/{match_id}/loadouts")
        } else {
            format!("/pregame/v1/matches/{match_id}/loadouts")
        };
        let data = self.glz_get(&path).await?;
        let entries = data
            .get("Loadouts")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut out = HashMap::new();
        for entry in entries {
            let subject = entry
                .get("Subject")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase();
            let items = entry
                .get("Loadout")
                .and_then(|l| l.get("Items"))
                .or_else(|| entry.get("Items"))
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            if !subject.is_empty() && !items.is_empty() {
                out.insert(subject, items);
            }
        }
        Ok(out)
    }

    pub async fn reveal_names(&self, puuids: &[String]) -> HashMap<String, String> {
        let mut names = HashMap::new();
        if puuids.is_empty() {
            return names;
        }
        if let Ok(value) = self
            .pd_put("/name-service/v2/players", &json!(puuids))
            .await
        {
            ingest_names(&mut names, &value);
        }
        let missing: Vec<String> = puuids
            .iter()
            .filter(|puuid| !names.contains_key(*puuid))
            .take(3)
            .cloned()
            .collect();
        for puuid in missing {
            if let Ok(value) = self
                .pd_put("/name-service/v2/players", &json!([puuid]))
                .await
            {
                ingest_names(&mut names, &value);
            }
        }
        names
    }

    pub async fn seasons(&self) -> Vec<Value> {
        for _ in 0..2 {
            if let Ok(data) = self
                .shared_get("/content-service/v3/content")
                .await
            {
                if let Some(seasons) = data.get("Seasons").and_then(Value::as_array) {
                    if !seasons.is_empty() {
                        return seasons.clone();
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(120)).await;
        }
        Vec::new()
    }

    pub fn active_season(seasons: &[Value]) -> Option<String> {
        seasons.iter().find_map(|season| {
            if season.get("IsActive").and_then(Value::as_bool) == Some(true)
                && season.get("Type").and_then(Value::as_str) == Some("act")
            {
                season.get("ID").and_then(Value::as_str).map(str::to_string)
            } else {
                None
            }
        })
    }

    pub fn previous_season(seasons: &[Value]) -> Option<String> {
        let current = seasons.iter().find(|season| {
            season.get("IsActive").and_then(Value::as_bool) == Some(true)
                && season.get("Type").and_then(Value::as_str) == Some("act")
        })?;
        let start = current.get("StartTime").and_then(Value::as_str)?;
        seasons.iter().find_map(|season| {
            if season.get("Type").and_then(Value::as_str) == Some("act")
                && season.get("EndTime").and_then(Value::as_str) == Some(start)
            {
                season.get("ID").and_then(Value::as_str).map(str::to_string)
            } else {
                None
            }
        })
    }

    pub async fn rank_info(
        &self,
        puuid: &str,
        season: Option<&str>,
        previous_season: Option<&str>,
    ) -> RankInfo {
        if let Ok(mmr) = self.pd_get(&format!("/mmr/v1/players/{puuid}")).await {
            return parse_rank(&mmr, season, previous_season);
        }
        RankInfo::default()
    }

    pub async fn fresh_match_ids(&self, puuid: &str) -> (Vec<String>, bool) {
        for queue in ["competitive", "unrated", "swiftplay", ""] {
            let suffix = if queue.is_empty() {
                String::new()
            } else {
                format!("&queue={queue}")
            };
            let path = format!(
                "/match-history/v1/history/{puuid}?startIndex=0&endIndex=10{suffix}"
            );
            if let Ok(history) = self.pd_get(&path).await {
                if let Some(entries) = history.get("History").and_then(Value::as_array) {
                    let mids = entries
                        .iter()
                        .filter_map(|entry| entry.get("MatchID").and_then(Value::as_str))
                        .map(str::to_string)
                        .collect::<Vec<_>>();
                    let is_comp = queue == "competitive" && !mids.is_empty();
                    return (mids, is_comp);
                }
                if history.get("status").and_then(Value::as_i64) == Some(429) {
                    return (Vec::new(), false);
                }
            }
        }
        (Vec::new(), false)
    }

    pub async fn match_detail(&self, match_id: &str) -> Result<Value, RiotError> {
        self.pd_get(&format!("/match-details/v1/matches/{match_id}"))
            .await
    }

    pub async fn party_snapshot(&self) -> Result<Value, RiotError> {
        let private = self
            .self_private(&self.presences().await?)
            .ok_or_else(|| RiotError::NotReady("no self presence".into()))?;
        let pdata = private.get("partyPresenceData").unwrap_or(&private);
        let Some(party_id) = pdata
            .get("partyId")
            .or_else(|| private.get("partyId"))
            .and_then(Value::as_str)
        else {
            return Ok(json!({ "available": false, "message": "Not in a party" }));
        };
        let queue_id = private
            .get("queueId")
            .or_else(|| {
                private
                    .get("matchPresenceData")
                    .and_then(|m| m.get("queueId"))
            })
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let mut snapshot = json!({
            "available": true,
            "partyId": party_id,
            "queueId": if queue_id.is_empty() { Value::Null } else { json!(queue_id) },
            "queueName": if queue_id.is_empty() { Value::Null } else { json!(catalog::mode_label(&queue_id)) },
            "eligible": [],
            "state": pdata.get("partyState").and_then(Value::as_str).unwrap_or("DEFAULT"),
            "inQueue": false,
            "partySize": pdata.get("partySize").and_then(Value::as_u64).unwrap_or(1),
            "isOwner": pdata.get("isPartyOwner").and_then(Value::as_bool).unwrap_or(true),
            "allReady": true,
            "queuedAt": Value::Null
        });
        if let Ok(party) = self.glz_get(&format!("/parties/v1/parties/{party_id}")).await {
            let members = party
                .get("Members")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if !members.is_empty() {
                if let Some(gqid) = party
                    .get("MatchmakingData")
                    .and_then(|m| m.get("QueueID"))
                    .and_then(Value::as_str)
                {
                    snapshot["queueId"] = json!(gqid.to_lowercase());
                    snapshot["queueName"] = json!(catalog::mode_label(gqid));
                }
                if let Some(state) = party.get("State").and_then(Value::as_str) {
                    snapshot["state"] = json!(state);
                    snapshot["inQueue"] = json!(state.contains("MATCHMAKING"));
                }
                snapshot["eligible"] = json!(party
                    .get("EligibleQueues")
                    .and_then(Value::as_array)
                    .map(|queues| queues
                        .iter()
                        .filter_map(|queue| queue.as_str())
                        .map(|queue| json!({
                            "id": queue,
                            "name": catalog::mode_label(queue)
                        }))
                        .collect::<Vec<_>>())
                    .unwrap_or_default());
                snapshot["partySize"] = json!(members.len());
                snapshot["allReady"] = json!(members.iter().all(|member| {
                    member.get("IsReady").and_then(Value::as_bool).unwrap_or(true)
                }));
                if let Some(me) = members.iter().find(|member| {
                    member.get("Subject").and_then(Value::as_str) == Some(self.puuid.as_str())
                }) {
                    snapshot["isOwner"] = json!(me
                        .get("IsOwner")
                        .and_then(Value::as_bool)
                        .unwrap_or(false));
                }
            }
        }
        Ok(snapshot)
    }

    pub async fn own_party_subjects(&self, presences: &[Value]) -> Vec<String> {
        self.own_party_members(presences)
            .await
            .into_iter()
            .filter_map(|member| {
                member
                    .get("Subject")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect()
    }

    pub async fn own_party_members(&self, presences: &[Value]) -> Vec<Value> {
        let Some(private) = self.self_private(presences) else {
            return Vec::new();
        };
        let pdata = private.get("partyPresenceData").unwrap_or(&private);
        let Some(party_id) = pdata
            .get("partyId")
            .or_else(|| private.get("partyId"))
            .and_then(Value::as_str)
        else {
            return Vec::new();
        };
        if let Ok(party) = self.glz_get(&format!("/parties/v1/parties/{party_id}")).await {
            return party
                .get("Members")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
        }
        Vec::new()
    }
}

fn ingest_names(names: &mut HashMap<String, String>, value: &Value) {
    let rows = value
        .as_array()
        .cloned()
        .or_else(|| value.get("players").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    for row in rows {
        let subject = row.get("Subject").and_then(Value::as_str).unwrap_or("");
        let game = row.get("GameName").and_then(Value::as_str).unwrap_or("");
        let tag = row.get("TagLine").and_then(Value::as_str).unwrap_or("");
        if !subject.is_empty() && !game.trim().is_empty() {
            names.insert(
                subject.to_string(),
                if tag.is_empty() {
                    game.to_string()
                } else {
                    format!("{game}#{tag}")
                },
            );
        }
    }
}

pub fn decode_private(raw: &str) -> Option<Value> {
    if raw.trim_start().starts_with('{') {
        return serde_json::from_str(raw).ok();
    }
    let bytes = B64.decode(raw).ok()?;
    serde_json::from_slice(&bytes).ok()
}

async fn decode(response: Response) -> Result<Value, RiotError> {
    let status = response.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(RiotError::RateLimited);
    }
    let text = response.text().await?;
    let value: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(RiotError::NotReady(format!("Riot returned {status}")));
        }
        if status.as_u16() == 404 {
            return Ok(value);
        }
        return Err(RiotError::Other(format!("Riot returned {status}")));
    }
    Ok(value)
}

fn parse_rank(mmr: &Value, season: Option<&str>, previous_season: Option<&str>) -> RankInfo {
    let Some(skills) = mmr.get("QueueSkills") else {
        return RankInfo::default();
    };
    let seasons = skills
        .get("competitive")
        .and_then(|c| c.get("SeasonalInfoBySeasonID"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let current = season
        .and_then(|id| seasons.get(id))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut info = RankInfo {
        tier: current
            .get("CompetitiveTier")
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32,
        rr: current
            .get("RankedRating")
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32,
        leaderboard: current
            .get("LeaderboardRank")
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32,
        peak: 0,
        win_rate: 0,
        games: current
            .get("NumberOfGames")
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32,
        previous: previous_season
            .and_then(|id| seasons.get(id))
            .and_then(|s| s.get("CompetitiveTier"))
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32,
        peak_season: season.unwrap_or("").to_string(),
        ok: true,
    };
    let wins = current
        .get("NumberOfWinsWithPlacements")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    info.win_rate = if info.games > 0 {
        ((wins as f64 / info.games as f64) * 100.0).round() as i32
    } else {
        0
    };
    info.peak = info.tier;
    for (season_id, season_info) in &seasons {
        if let Some(wins_by_tier) = season_info.get("WinsByTier").and_then(Value::as_object) {
            for tier in wins_by_tier.keys() {
                let Ok(mut value) = tier.parse::<i32>() else {
                    continue;
                };
                if BEFORE_ASCENDANT.contains(&season_id.to_lowercase().as_str()) && value > 20 {
                    value += 3;
                }
                if value > info.peak {
                    info.peak = value;
                    info.peak_season = season_id.clone();
                }
            }
        }
    }
    info
}

const BEFORE_ASCENDANT: &[&str] = &[
    "0df5adb9-4dcb-6899-1306-3e9860661dd3",
    "3f61c772-4560-cd3f-5d3f-a7ab5abda6b3",
    "0530b9c4-4980-f2ee-df5d-09864cd00542",
    "46ea6166-4573-1128-9cea-60a15640059b",
    "fcf2c8f4-4324-e50b-2e23-718e4a3ab046",
    "97b6e739-44cc-ffa7-49ad-398ba502ceb0",
    "ab57ef51-4e59-da91-cc8d-51a5a2b9b8ff",
    "52e9749a-429b-7060-99fe-4595426a0cf7",
    "71c81c67-4fae-ceb1-844c-aab2bb8710fa",
    "2a27e5d2-4d30-c9e2-b15a-93b8909a442c",
    "4cb622e1-4244-6da3-7276-8daaf1c01be2",
    "a16955a5-4ad0-f761-5e9e-389df1c892fb",
    "97b39124-46ce-8b55-8fd1-7cbf7ffe173f",
    "573f53ac-41a5-3a7d-d9ce-d6a6298e5704",
    "d929bc38-4ab6-7da4-94f0-ee84f8ac141e",
    "3e47230a-463c-a301-eb7d-67bb60357d4f",
    "808202d6-4f2b-a8ff-1feb-b3a0590ad79f",
];

pub struct RiotRuntime {
    http: Client,
    rate: Arc<RateLimiter>,
    cached: Mutex<Option<(Arc<Session>, Instant, String)>>,
}

impl RiotRuntime {
    pub fn new(http: Client) -> Self {
        Self {
            http,
            rate: Arc::new(RateLimiter::new()),
            cached: Mutex::new(None),
        }
    }

    pub async fn session(&self, region_override: &str) -> Result<Arc<Session>, RiotError> {
        let lockfile = read_lockfile()?;
        let key = format!("{}:{}:{}", lockfile.port, lockfile.password, lockfile.pid);
        {
            let cache = self.cached.lock().await;
            if let Some((session, created, cached_key)) = cache.as_ref() {
                if cached_key == &key && created.elapsed() < Duration::from_secs(15 * 60) {
                    return Ok(session.clone());
                }
            }
        }
        let region = resolve_region(region_override)?;
        let auth_header = format!("Basic {}", B64.encode(format!("riot:{}", lockfile.password)));
        let response = self
            .http
            .get(format!(
                "https://127.0.0.1:{}/entitlements/v1/token",
                lockfile.port
            ))
            .header(AUTHORIZATION, auth_header)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(RiotError::NotReady(format!(
                "entitlements returned {}",
                response.status()
            )));
        }
        let entitlements: Value = response.json().await?;
        let puuid = entitlements
            .get("subject")
            .and_then(Value::as_str)
            .ok_or_else(|| RiotError::NotReady("no account subject".into()))?
            .to_string();
        let access = entitlements
            .get("accessToken")
            .and_then(Value::as_str)
            .ok_or_else(|| RiotError::NotReady("no access token".into()))?;
        let token = entitlements
            .get("token")
            .and_then(Value::as_str)
            .ok_or_else(|| RiotError::NotReady("no entitlements token".into()))?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {access}"))
                .map_err(|e| RiotError::Other(e.to_string()))?,
        );
        headers.insert(
            "X-Riot-Entitlements-JWT",
            HeaderValue::from_str(token).map_err(|e| RiotError::Other(e.to_string()))?,
        );
        headers.insert("X-Riot-ClientPlatform", HeaderValue::from_static(PLATFORM));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(USER_AGENT, HeaderValue::from_static(SHARED_GAME_CLIENT));

        let provisional = Session {
            http: self.http.clone(),
            rate: self.rate.clone(),
            lockfile: lockfile.clone(),
            puuid: puuid.clone(),
            region: region.clone(),
            headers: headers.clone(),
            client_version: String::new(),
            created: Instant::now(),
        };
        let version = resolve_client_version(&provisional).await;
        let mut final_headers = headers;
        if let Ok(value) = HeaderValue::from_str(&version) {
            final_headers.insert("X-Riot-ClientVersion", value);
        }
        final_headers.insert(USER_AGENT, HeaderValue::from_static(SHARED_GAME_CLIENT));
        let session = Arc::new(Session {
            http: self.http.clone(),
            rate: self.rate.clone(),
            lockfile,
            puuid,
            region,
            headers: final_headers,
            client_version: version,
            created: Instant::now(),
        });
        let created = Instant::now();
        *self.cached.lock().await = Some((session.clone(), created, key));
        Ok(session)
    }

    pub async fn invalidate(&self) {
        *self.cached.lock().await = None;
    }
}

async fn resolve_client_version(session: &Session) -> String {
    if let Ok(value) = session.local_get("/chat/v4/presences").await {
        if let Some(rows) = value.get("presences").and_then(Value::as_array) {
            for presence in rows {
                if presence.get("product").and_then(Value::as_str) != Some("valorant") {
                    continue;
                }
                let Some(private) = presence.get("private").and_then(Value::as_str) else {
                    continue;
                };
                if let Some(decoded) = decode_private(private) {
                    let version = decoded
                        .get("partyPresenceData")
                        .and_then(|p| p.get("partyClientVersion"))
                        .or_else(|| decoded.get("partyClientVersion"))
                        .and_then(Value::as_str);
                    if let Some(version) = version {
                        if version != "unknown" {
                            return version.to_string();
                        }
                    }
                }
            }
        }
    }
    if let Ok(response) = session
        .http
        .get("https://valorant-api.com/v1/version")
        .timeout(Duration::from_secs(6))
        .send()
        .await
    {
        if let Ok(value) = response.json::<Value>().await {
            if let Some(version) = value
                .get("data")
                .and_then(|d| d.get("riotClientVersion"))
                .and_then(Value::as_str)
            {
                return version.to_string();
            }
        }
    }
    if let Ok(text) = std::fs::read_to_string(shooter_log_path()) {
        for line in text.lines().rev() {
            if let Some((_, version)) = line.split_once("CI server version:") {
                return version.trim().to_string();
            }
        }
    }
    "release-09.00".into()
}

pub fn read_lockfile() -> Result<Lockfile, RiotError> {
    let path = lockfile_path();
    let text = std::fs::read_to_string(path).map_err(|_| RiotError::ClientUnavailable)?;
    let mut parts = text.split(':');
    let name = parts.next().unwrap_or("").to_string();
    let pid = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let port = parts
        .next()
        .and_then(|p| p.parse().ok())
        .ok_or(RiotError::Lockfile)?;
    let password = parts.next().unwrap_or("").to_string();
    let protocol = parts.next().unwrap_or("https").to_string();
    if !process_exists(pid) {
        return Err(RiotError::ClientUnavailable);
    }
    Ok(Lockfile {
        name,
        pid,
        port,
        password,
        protocol,
    })
}

pub fn lockfile_path() -> PathBuf {
    local_appdata()
        .join("Riot Games")
        .join("Riot Client")
        .join("Config")
        .join("lockfile")
}

pub fn shooter_log_path() -> PathBuf {
    local_appdata()
        .join("VALORANT")
        .join("Saved")
        .join("Logs")
        .join("ShooterGame.log")
}

pub fn local_appdata() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_default()
}

pub fn client_status() -> ClientStatus {
    let path = lockfile_path();
    let lockfile = read_lockfile();
    let available = lockfile.is_ok();
    ClientStatus {
        available,
        lockfile_path: path.display().to_string(),
        region: detect_region().map(|r| r.label()),
        error: if available {
            None
        } else {
            Some("Riot Client is not running".into())
        },
    }
}

pub fn detect_region() -> Option<RegionInfo> {
    let text = std::fs::read_to_string(shooter_log_path()).ok()?;
    let mut pd = None;
    let mut glz = None;
    for line in text.lines().rev().take(400_000) {
        if pd.is_none() && line.contains(".a.pvp.net/account-xp/v1/") {
            pd = line
                .split(".a.pvp.net/account-xp/v1/")
                .next()
                .and_then(|left| left.split('.').last())
                .map(str::to_string);
        }
        if glz.is_none() && line.contains("https://glz-") {
            if let Some(rest) = line.split("https://glz-").nth(1) {
                let mut parts = rest.split('.');
                let a = parts.next()?;
                let b = parts.next()?;
                glz = Some((a.to_string(), b.to_string()));
            }
        }
        if pd.is_some() && glz.is_some() {
            break;
        }
    }
    let shard = pd?;
    let (glz_a, glz_b) = glz.unwrap_or_else(|| (format!("{shard}-1"), shard.clone()));
    Some(RegionInfo {
        shard: if shard == "pbe" { "na".into() } else { shard },
        glz_a,
        glz_b,
    })
}

pub fn resolve_region(override_region: &str) -> Result<RegionInfo, RiotError> {
    let key = override_region.trim().to_lowercase();
    let mapped = match key.as_str() {
        "na" => Some(("na", "na-1", "na")),
        "eu" => Some(("eu", "eu-1", "eu")),
        "ap" => Some(("ap", "ap-1", "ap")),
        "kr" => Some(("kr", "kr-1", "kr")),
        "latam" => Some(("na", "latam-1", "na")),
        "br" => Some(("na", "br-1", "na")),
        _ => None,
    };
    if let Some((shard, a, b)) = mapped {
        return Ok(RegionInfo {
            shard: shard.into(),
            glz_a: a.into(),
            glz_b: b.into(),
        });
    }
    detect_region().ok_or_else(|| RiotError::Other("could not detect region".into()))
}

pub fn process_exists(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return false;
            }
            let mut code = 0u32;
            let ok = GetExitCodeProcess(handle, &mut code) != 0;
            CloseHandle(handle);
            ok && code == STILL_ACTIVE as u32
        }
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        true
    }
}

struct Buckets {
    general: f64,
    general_at: Instant,
    mmr: f64,
    mmr_at: Instant,
    hold_until: Instant,
    mmr_hold_until: Instant,
}

pub struct RateLimiter {
    state: Mutex<Buckets>,
}

impl RateLimiter {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            state: Mutex::new(Buckets {
                general: 10.0,
                general_at: now,
                mmr: 24.0,
                mmr_at: now,
                hold_until: now,
                mmr_hold_until: now,
            }),
        }
    }

    pub async fn acquire(&self, mmr: bool) {
        loop {
            let wait = {
                let mut buckets = self.state.lock().await;
                let now = Instant::now();
                let elapsed = now.duration_since(buckets.general_at).as_secs_f64();
                buckets.general = (10.0f64).min(buckets.general + elapsed * 10.0);
                buckets.general_at = now;
                let mmr_elapsed = now.duration_since(buckets.mmr_at).as_secs_f64();
                buckets.mmr = (24.0f64).min(buckets.mmr + mmr_elapsed * 0.4);
                buckets.mmr_at = now;
                if now < buckets.hold_until {
                    buckets.hold_until.duration_since(now).as_secs_f64()
                } else if mmr && now < buckets.mmr_hold_until {
                    buckets.mmr_hold_until.duration_since(now).as_secs_f64()
                } else if buckets.general < 1.0 {
                    (1.0 - buckets.general) / 10.0
                } else if mmr && buckets.mmr < 1.0 {
                    (1.0 - buckets.mmr) / 0.4
                } else {
                    buckets.general -= 1.0;
                    if mmr {
                        buckets.mmr -= 1.0;
                    }
                    0.0
                }
            };
            if wait <= 0.0 {
                return;
            }
            tokio::time::sleep(Duration::from_secs_f64(wait.max(0.02))).await;
        }
    }

    pub async fn hold(&self, seconds: f64, mmr: bool) {
        let mut buckets = self.state.lock().await;
        let until = Instant::now() + Duration::from_secs_f64(seconds.max(0.0));
        if mmr {
            buckets.mmr_hold_until = buckets.mmr_hold_until.max(until);
        } else {
            buckets.hold_until = buckets.hold_until.max(until);
        }
    }
}
