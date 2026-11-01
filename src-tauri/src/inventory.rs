use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use serde_json::{json, Value};

use crate::{app::AppState, catalog, history::now};

const ITEM_TYPES: &[(&str, &str)] = &[
    ("skins", "e7c63390-eda7-46e0-bb7a-a6abdacd2433"),
    ("buddies", "dd3bf334-87f3-40bd-b043-682a57a8dc3a"),
    ("cards", "3f296c07-64c3-494c-923b-fe692a4fa1bd"),
    ("sprays", "d5f120f8-ff8c-4aac-92ea-f2b5acbe9475"),
    ("agents", "01bb38e1-da47-4e6a-9b3d-945fe4655707"),
];
const CUR_VP: &str = "85ad13f7-3d1b-5128-9eb2-7cd8ee0b5741";
const CUR_RAD: &str = "e59aa87c-4cbf-517a-5983-6e81511be9b7";
const CUR_KC: &str = "85ca954a-41f2-ce94-9b45-8ca3dd39a00d";
const VP_PER_USD: f64 = 107.0;

pub async fn snapshot(state: &Arc<AppState>, refresh: bool) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let cache_key = format!("inventory:{}", session.puuid);
    if !refresh {
        if let Some(cached) = state.meta.lock().get(&cache_key) {
            if let Some(at) = cached.get("cachedAt").and_then(Value::as_u64) {
                if now().saturating_sub(at) < 600 {
                    return Ok(cached.clone());
                }
            }
        }
    }
    let account = session
        .local_get("/chat/v1/session")
        .await
        .ok()
        .and_then(|value| {
            let name = value.get("game_name").and_then(Value::as_str).unwrap_or("");
            let tag = value.get("game_tag").and_then(Value::as_str).unwrap_or("");
            if name.is_empty() {
                return None;
            }
            Some(if tag.is_empty() {
                name.to_string()
            } else {
                format!("{name}#{tag}")
            })
        })
        .unwrap_or_else(|| session.puuid.clone());
    let base = state.valo.base_skin_levels().await;
    if base.is_empty() {
        return Err("The skin catalog could not be loaded.".into());
    }
    let freebies = state.valo.contract_rewards().await;
    let freebies: std::collections::HashSet<String> = freebies.into_iter().collect();
    let entitlements = fetch_entitlements(&session, &session.puuid).await;
    let Some(entitlements) = entitlements else {
        return Err("The Riot client could not read your collection.".into());
    };
    let owned = entitlements
        .get("skins")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut total = 0i64;
    let mut earned = 0i64;
    let mut priced: Vec<Value> = Vec::new();
    let mut owned_items: Vec<Value> = Vec::new();
    for skin_id in owned {
        let Some(id) = skin_id.as_str() else {
            continue;
        };
        let Some(meta) = base.get(&id.to_lowercase()) else {
            continue;
        };
        let mut item = meta.clone();
        item["uuid"] = json!(id);
        if freebies.contains(&id.to_lowercase()) {
            earned += 1;
            item["vp"] = json!(0);
            item["earned"] = json!(true);
            owned_items.push(json!({
                "type": "skin",
                "uuid": item.get("uuid").cloned().unwrap_or(Value::Null),
                "name": item.get("name").cloned().unwrap_or(Value::Null),
                "icon": item.get("icon").cloned().unwrap_or(Value::Null),
                "tier": item.get("tier").cloned().unwrap_or(Value::Null),
                "vp": 0,
                "earned": true
            }));
            continue;
        }
        total += meta.get("vp").and_then(Value::as_i64).unwrap_or(0);
        item["earned"] = json!(false);
        owned_items.push(json!({
            "type": "skin",
            "uuid": item.get("uuid").cloned().unwrap_or(Value::Null),
            "name": item.get("name").cloned().unwrap_or(Value::Null),
            "icon": item.get("icon").cloned().unwrap_or(Value::Null),
            "tier": item.get("tier").cloned().unwrap_or(Value::Null),
            "vp": item.get("vp").cloned().unwrap_or(Value::Null),
            "earned": false
        }));
        priced.push(item);
    }
    let buddy_meta = state.valo.buddy_meta().await;
    let card_meta = state.valo.card_meta().await;
    let spray_meta = state.valo.spray_meta().await;
    for (kind, key, meta) in [
        ("buddy", "buddies", &buddy_meta),
        ("card", "cards", &card_meta),
        ("spray", "sprays", &spray_meta),
    ] {
        let Some(ids) = entitlements.get(key).and_then(Value::as_array) else {
            continue;
        };
        for id in ids {
            let Some(uuid) = id.as_str() else {
                continue;
            };
            let lookup = uuid.to_lowercase();
            let entry = meta.get(&lookup);
            let name = entry
                .and_then(|value| value.get("name"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| uuid.to_string());
            let icon = entry
                .and_then(|value| value.get("icon"))
                .cloned()
                .unwrap_or(Value::Null);
            owned_items.push(json!({
                "type": kind,
                "uuid": uuid,
                "name": name,
                "icon": icon
            }));
        }
    }
    if let Some(ids) = entitlements.get("agents").and_then(Value::as_array) {
        for id in ids {
            let Some(uuid) = id.as_str() else {
                continue;
            };
            owned_items.push(json!({
                "type": "agent",
                "uuid": uuid,
                "name": catalog::agent_name_by_uuid(uuid).unwrap_or("Agent"),
                "icon": format!("https://media.valorant-api.com/agents/{uuid}/displayicon.png")
            }));
        }
    }
    let mut seen = std::collections::HashSet::new();
    owned_items.retain(|item| {
        let key = format!(
            "{}:{}",
            item.get("type").and_then(Value::as_str).unwrap_or(""),
            item.get("uuid").and_then(Value::as_str).unwrap_or("")
        );
        seen.insert(key)
    });
    owned_items.sort_by_key(|item| {
        let rank = match item.get("type").and_then(Value::as_str).unwrap_or("") {
            "skin" => 0,
            "agent" => 1,
            "buddy" => 2,
            "card" => 3,
            "spray" => 4,
            _ => 5,
        };
        (
            rank,
            item.get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase(),
        )
    });
    priced.sort_by_key(|skin| -skin.get("vp").and_then(Value::as_i64).unwrap_or(0));
    let mut tiers: HashMap<String, Value> = HashMap::new();
    for skin in &priced {
        let tier = skin
            .get("tier")
            .and_then(Value::as_str)
            .unwrap_or("Other")
            .to_string();
        let entry = tiers.entry(tier).or_insert_with(|| json!({ "skins": 0, "vp": 0 }));
        entry["skins"] = json!(entry.get("skins").and_then(Value::as_i64).unwrap_or(0) + 1);
        entry["vp"] = json!(
            entry.get("vp").and_then(Value::as_i64).unwrap_or(0)
                + skin.get("vp").and_then(Value::as_i64).unwrap_or(0)
        );
    }
    let wallet = session
        .pd_get(&format!("/store/v1/wallet/{}", session.puuid))
        .await
        .ok()
        .and_then(|value| value.get("Balances").cloned())
        .unwrap_or_else(|| json!({}));
    let mut counts = serde_json::Map::new();
    counts.insert("skins".into(), json!(priced.len()));
    counts.insert("earned".into(), json!(earned));
    for (name, kind) in [
        ("buddies", "buddy"),
        ("cards", "card"),
        ("sprays", "spray"),
        ("agents", "agent"),
    ] {
        let count = owned_items
            .iter()
            .filter(|item| item.get("type").and_then(Value::as_str) == Some(kind))
            .count();
        counts.insert(name.into(), json!(count));
    }
    let top: Vec<Value> = priced.iter().take(12).cloned().collect();
    let value = json!({
        "available": true,
        "account": account,
        "totalVp": total,
        "usdApprox": (total as f64 / VP_PER_USD).round(),
        "wallet": {
            "vp": wallet.get(CUR_VP).and_then(Value::as_i64).unwrap_or(0),
            "rad": wallet.get(CUR_RAD).and_then(Value::as_i64).unwrap_or(0),
            "kc": wallet.get(CUR_KC).and_then(Value::as_i64).unwrap_or(0)
        },
        "counts": counts,
        "tiers": tiers,
        "top": top,
        "owned": owned_items,
        "cachedAt": now()
    });
    state
        .meta
        .lock()
        .insert(cache_key, value.clone());
    Ok(value)
}

async fn fetch_entitlements(
    session: &crate::riot::Session,
    puuid: &str,
) -> Option<HashMap<String, Value>> {
    let requests = ITEM_TYPES.iter().map(|(name, item_type)| {
        let session = session.clone();
        let puuid = puuid.to_string();
        let name = name.to_string();
        let item_type = item_type.to_string();
        async move {
            let data = session
                .pd_get(&format!("/store/v1/entitlements/{puuid}/{item_type}"))
                .await
                .ok();
            let ok = data
                .as_ref()
                .and_then(|value| value.get("Entitlements"))
                .is_some();
            let ids: Vec<Value> = data
                .as_ref()
                .and_then(|value| value.get("Entitlements"))
                .and_then(Value::as_array)
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|entry| entry.get("ItemID").cloned())
                        .collect()
                })
                .unwrap_or_default();
            (name, json!(ids), ok)
        }
    });
    let rows = join_all(requests).await;
    if !rows.iter().any(|(_, _, ok)| *ok) {
        return None;
    }
    Some(
        rows.into_iter()
            .map(|(name, ids, _)| (name, ids))
            .collect(),
    )
}


