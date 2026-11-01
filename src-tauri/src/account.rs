use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use crate::{app::AppState, content, riot::Session};

fn parse_user_info(raw: &Value) -> Value {
    raw.get("userInfo")
        .and_then(Value::as_str)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .unwrap_or_else(|| json!({}))
}

pub async fn snapshot(state: &Arc<AppState>) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let cache_key = format!("account:overview:{}", session.puuid);
    if let Some(cached) = state.cache_json_for(&cache_key, Duration::from_secs(120)) {
        return Ok(cached);
    }
    let alias = session
        .local_get("/player-account/aliases/v1/active")
        .await
        .unwrap_or_else(|_| json!({}));
    let rso = parse_user_info(
        &session
            .local_get("/rso-auth/v1/authorization/userinfo")
            .await
            .unwrap_or_else(|_| json!({})),
    );
    let xp = session
        .pd_get(&format!("/account-xp/v1/players/{}", session.puuid))
        .await
        .map_err(|error| format!("Account XP unavailable: {error}"))?;
    let wallet = session
        .pd_get(&format!("/store/v1/wallet/{}", session.puuid))
        .await
        .ok()
        .and_then(|value| value.get("Balances").cloned())
        .unwrap_or_else(|| json!({}));
    let penalties = session
        .pd_get("/restrictions/v3/penalties")
        .await
        .ok()
        .and_then(|value| value.get("Penalties").cloned())
        .unwrap_or_else(|| json!([]));
    let seasons = session.seasons().await;
    let active = Session::active_season(&seasons);
    let previous = Session::previous_season(&seasons);
    let rank = session
        .rank_info(
            &session.puuid,
            active.as_deref(),
            previous.as_deref(),
        )
        .await;
    let account = rso.get("acct").cloned().unwrap_or_else(|| json!({}));
    let affinity = rso
        .get("affinity")
        .and_then(|value| value.get("pp"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let providers = rso
        .get("federated_identity_details")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("provider_name").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let history = xp
        .get("History")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .rev()
                .take(12)
                .map(|row| {
                    let sources = row
                        .get("XPSources")
                        .and_then(Value::as_array)
                        .map(|sources| {
                            sources
                                .iter()
                                .filter_map(|source| {
                                    let id = source.get("ID").and_then(Value::as_str)?;
                                    let amount = source.get("Amount").and_then(Value::as_i64)?;
                                    Some(json!({ "id": id, "amount": amount }))
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    json!({
                        "matchId": row.get("ID").cloned().unwrap_or(Value::Null),
                        "startedAt": row.get("MatchStart").cloned().unwrap_or(Value::Null),
                        "xp": row.get("XPDelta").and_then(Value::as_i64).unwrap_or(0),
                        "level": row.get("EndProgress").and_then(|value| value.get("Level")).cloned().unwrap_or(Value::Null),
                        "sources": sources
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let value = json!({
        "available": true,
        "identity": {
            "gameName": alias.get("game_name").cloned().unwrap_or_else(|| account.get("game_name").cloned().unwrap_or(Value::Null)),
            "tagLine": alias.get("tag_line").cloned().unwrap_or_else(|| account.get("tag_line").cloned().unwrap_or(Value::Null)),
            "createdAt": alias.get("created_datetime").cloned().unwrap_or_else(|| account.get("created_at").cloned().unwrap_or(Value::Null)),
            "country": rso.get("country").cloned().unwrap_or(Value::Null),
            "locale": rso.get("player_locale").cloned().unwrap_or(Value::Null),
            "ageCategory": rso.get("age_category").cloned().unwrap_or(Value::Null),
            "state": account.get("state").cloned().unwrap_or(Value::Null),
            "accountVerified": rso.get("account_verified").cloned().unwrap_or(json!(false)),
            "emailVerified": rso.get("email_verified").cloned().unwrap_or(json!(false)),
            "phoneVerified": rso.get("phone_number_verified").cloned().unwrap_or(json!(false)),
            "providers": providers,
            "affinity": affinity
        },
        "progress": {
            "level": xp.get("Progress").and_then(|value| value.get("Level")).and_then(Value::as_i64).unwrap_or(0),
            "xp": xp.get("Progress").and_then(|value| value.get("XP")).and_then(Value::as_i64).unwrap_or(0),
            "lastFirstWin": xp.get("LastTimeGrantedFirstWin").cloned().unwrap_or(Value::Null),
            "nextFirstWin": xp.get("NextTimeFirstWinAvailable").cloned().unwrap_or(Value::Null)
        },
        "history": history,
        "rank": rank,
        "wallet": {
            "vp": wallet.get(content::CURRENCY_VP).and_then(Value::as_i64).unwrap_or(0),
            "rad": wallet.get(content::CURRENCY_RAD).and_then(Value::as_i64).unwrap_or(0),
            "kc": wallet.get(content::CURRENCY_KC).and_then(Value::as_i64).unwrap_or(0)
        },
        "penalties": penalties,
        "region": session.region.shard,
        "clientVersion": session.client_version,
        "accountLabel": content::account_label(&session).await
    });
    state.put_json(cache_key, value.clone());
    Ok(value)
}
