use std::{collections::HashMap, sync::Arc};

use futures::future::join_all;
use serde_json::{json, Value};

use crate::{app::AppState, history::now};

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

pub async fn snapshot(state: &Arc<AppState>) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let cache_key = format!("inventory:{}", session.puuid);
    if let Some(cached) = state.meta.lock().get(&cache_key) {
        if let Some(at) = cached.get("cachedAt").and_then(Value::as_u64) {
            if now().saturating_sub(at) < 600 {
                return Ok(cached.clone());
            }
        }
    }
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
    for skin_id in owned {
        let Some(id) = skin_id.as_str() else {
            continue;
        };
        let Some(meta) = base.get(&id.to_lowercase()) else {
            continue;
        };
        if freebies.contains(&id.to_lowercase()) {
            earned += 1;
            continue;
        }
        total += meta.get("vp").and_then(Value::as_i64).unwrap_or(0);
        priced.push(meta.clone());
    }
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
    for (name, item_type) in ITEM_TYPES {
        if *name == "skins" {
            continue;
        }
        let count = entitlements
            .get(*name)
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        counts.insert((*name).into(), json!(count));
        let _ = item_type;
    }
    let top: Vec<Value> = priced.iter().take(12).cloned().collect();
    let value = json!({
        "available": true,
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
            let value = session
                .pd_get(&format!("/store/v1/entitlements/{puuid}/{item_type}"))
                .await
                .ok()
                .and_then(|data| data.get("Entitlements").cloned())
                .unwrap_or_else(|| json!([]));
            let ids: Vec<Value> = value
                .as_array()
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|entry| entry.get("ItemID").cloned())
                        .collect()
                })
                .unwrap_or_default();
            (name, json!(ids))
        }
    });
    let rows = join_all(requests).await.into_iter().collect();
    Some(rows)
}


