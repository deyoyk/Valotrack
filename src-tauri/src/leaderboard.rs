use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use crate::{app::AppState, catalog, riot::Session};

pub async fn snapshot(
    state: &Arc<AppState>,
    query: Option<String>,
    size: Option<u32>,
) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let seasons = session.seasons().await;
    let season = Session::active_season(&seasons)
        .ok_or_else(|| "No active competitive act was found.".to_string())?;
    let size = size.unwrap_or(100).clamp(1, 510);
    let query = query.unwrap_or_default().trim().to_string();
    let encoded = urlencoding::encode(&query);
    let path = if query.is_empty() {
        format!(
            "/mmr/v1/leaderboards/affinity/{}/queue/competitive/season/{season}?startIndex=0&size={size}",
            session.region.shard
        )
    } else {
        format!(
            "/mmr/v1/leaderboards/affinity/{}/queue/competitive/season/{season}?startIndex=0&size={size}&query={encoded}",
            session.region.shard
        )
    };
    let cache_key = format!(
        "leaderboard:{}:{}:{}:{}",
        session.region.shard, season, size, query
    );
    if let Some(cached) = state.cache_json_for(&cache_key, Duration::from_secs(45)) {
        return Ok(cached);
    }
    let data = session
        .pd_get(&path)
        .await
        .map_err(|error| format!("Leaderboard unavailable: {error}"))?;
    let cards = state.valo.card_meta().await;
    let titles = state.valo.titles().await;
    let players = data
        .get("Players")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    let tier = row
                        .get("competitiveTier")
                        .and_then(Value::as_i64)
                        .unwrap_or(0) as i32;
                    let rank = catalog::rank_from_tier(tier);
                    let puuid = row.get("puuid").and_then(Value::as_str).unwrap_or("");
                    let card_id = row
                        .get("PlayerCardID")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_lowercase();
                    let title_id = row
                        .get("TitleID")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_lowercase();
                    let card = cards.get(&card_id);
                    json!({
                        "puuid": puuid,
                        "gameName": row.get("gameName").cloned().unwrap_or(Value::Null),
                        "tagLine": row.get("tagLine").cloned().unwrap_or(Value::Null),
                        "rank": row.get("leaderboardRank").and_then(Value::as_i64).unwrap_or(0),
                        "rr": row.get("rankedRating").and_then(Value::as_i64).unwrap_or(0),
                        "wins": row.get("numberOfWins").and_then(Value::as_i64).unwrap_or(0),
                        "tier": tier,
                        "tierName": rank.name,
                        "tierColor": rank.color,
                        "tierIcon": catalog::rank_icon(tier),
                        "card": card.and_then(|value| value.get("icon")).cloned().unwrap_or(Value::Null),
                        "title": titles.get(&title_id).cloned().unwrap_or_default(),
                        "isSelf": puuid == session.puuid,
                        "isBanned": row.get("IsBanned").and_then(Value::as_bool).unwrap_or(false),
                        "isAnonymized": row.get("IsAnonymized").and_then(Value::as_bool).unwrap_or(false)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let value = json!({
        "available": true,
        "season": season,
        "seasonLabel": state.valo.season_label(&season).await,
        "region": session.region.shard,
        "query": query,
        "size": size,
        "totalPlayers": data.get("totalPlayers").and_then(Value::as_i64).unwrap_or(0),
        "tierDetails": data.get("tierDetails").cloned().unwrap_or(Value::Null),
        "players": players
    });
    state.put_json(cache_key, value.clone());
    Ok(value)
}
