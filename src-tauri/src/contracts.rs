use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};

use crate::{
    app::AppState,
    catalog,
    content::{self, ItemIndex, ITEM_SKIN},
};

fn contract_map(rows: &[Value]) -> HashMap<String, Value> {
    rows.iter()
        .filter_map(|row| {
            let id = row.get("uuid").and_then(Value::as_str)?;
            Some((id.to_lowercase(), row.clone()))
        })
        .collect()
}

fn mission_map(rows: &[Value]) -> HashMap<String, Value> {
    rows.iter()
        .filter_map(|row| {
            let id = row.get("uuid").and_then(Value::as_str)?;
            Some((id.to_lowercase(), row.clone()))
        })
        .collect()
}

pub async fn snapshot(state: &Arc<AppState>) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let cache_key = format!("contracts:{}", session.puuid);
    if let Some(cached) = state.cache_json_for(&cache_key, Duration::from_secs(120)) {
        return Ok(cached);
    }
    let data = session
        .pd_get(&format!("/contracts/v1/contracts/{}", session.puuid))
        .await
        .map_err(|error| format!("Contracts unavailable: {error}"))?;
    let definitions = contract_map(&state.valo.list("contracts").await);
    let missions = mission_map(&state.valo.list("missions").await);
    let active = data
        .get("ActiveSpecialContract")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    let account = content::account_label(&session).await;

    let contracts = data
        .get("Contracts")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let id = row
                        .get("ContractDefinitionID")
                        .and_then(Value::as_str)?
                        .to_lowercase();
                    let definition = definitions.get(&id);
                    let relation_type = definition
                        .and_then(|value| value.get("content"))
                        .and_then(|value| value.get("relationType"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let relation_uuid = definition
                        .and_then(|value| value.get("content"))
                        .and_then(|value| value.get("relationUuid"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let agent = if relation_type == "Agent" {
                        catalog::agent_name_by_uuid(relation_uuid)
                    } else {
                        None
                    };
                    let name = definition
                        .and_then(|value| value.get("displayName"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .or_else(|| agent.map(str::to_string))
                        .unwrap_or_else(|| "Contract".into());
                    let icon = definition
                        .and_then(|value| value.get("displayIcon"))
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .or_else(|| {
                            catalog::agents()
                                .into_iter()
                                .find(|item| item.uuid.eq_ignore_ascii_case(relation_uuid))
                                .map(|item| item.portrait)
                        });
                    let progression = row
                        .get("ContractProgression")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    Some(json!({
                        "id": id,
                        "name": name,
                        "icon": icon,
                        "relationType": relation_type,
                        "relationUuid": relation_uuid,
                        "agent": agent,
                        "level": row.get("ProgressionLevelReached").and_then(Value::as_i64).unwrap_or(0),
                        "towardsNext": row.get("ProgressionTowardsNextLevel").and_then(Value::as_i64).unwrap_or(0),
                        "totalXp": progression.get("TotalProgressionEarned").and_then(Value::as_i64).unwrap_or(0),
                        "active": id == active
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let missions = data
        .get("Missions")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let id = row.get("ID").and_then(Value::as_str)?.to_lowercase();
                    let definition = missions.get(&id);
                    let title = definition
                        .and_then(|value| value.get("title"))
                        .and_then(Value::as_str)
                        .unwrap_or("Weekly mission");
                    let target = definition
                        .and_then(|value| value.get("progressToComplete"))
                        .and_then(Value::as_i64)
                        .unwrap_or(1);
                    let current = row
                        .get("Objectives")
                        .and_then(Value::as_object)
                        .and_then(|objectives| objectives.values().next())
                        .and_then(Value::as_i64)
                        .unwrap_or(0);
                    Some(json!({
                        "id": id,
                        "title": title,
                        "complete": row.get("Complete").and_then(Value::as_bool).unwrap_or(false),
                        "expires": row.get("ExpirationTime").cloned().unwrap_or(Value::Null),
                        "progress": current,
                        "target": target,
                        "xp": definition
                            .and_then(|value| value.get("xpGrant"))
                            .and_then(Value::as_i64)
                            .unwrap_or(0)
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let recent = data
        .get("ProcessedMatches")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .rev()
                .take(6)
                .map(|row| {
                    let xp = row.get("XPGrants").cloned().unwrap_or_else(|| json!({}));
                    json!({
                        "matchId": row.get("ID").cloned().unwrap_or(Value::Null),
                        "startedAt": row.get("StartTime").cloned().unwrap_or(Value::Null),
                        "gamePlayed": xp.get("GamePlayed").and_then(Value::as_i64).unwrap_or(0),
                        "gameWon": xp.get("GameWon").and_then(Value::as_i64).unwrap_or(0),
                        "roundPlayed": xp.get("RoundPlayed").and_then(Value::as_i64).unwrap_or(0),
                        "roundWon": xp.get("RoundWon").and_then(Value::as_i64).unwrap_or(0),
                        "contracts": row.get("ContractDeltas").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let upgrades = session
        .pd_get("/contract-definitions/v3/item-upgrades")
        .await
        .ok()
        .map(|value| value.get("Definitions").cloned().unwrap_or(json!([])))
        .unwrap_or_else(|| json!([]));
    let index = ItemIndex::build(&state.valo).await;
    let owned_skins = content::owned_ids(&session, ITEM_SKIN).await;
    let owned_skins = owned_skins.into_iter().collect::<std::collections::HashSet<_>>();
    let upgradeable = upgrades
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let required = row
                        .get("RequiredEntitlement")
                        .and_then(|value| value.get("ItemID"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let item_id = row
                        .get("Item")
                        .and_then(|value| value.get("ItemID"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    if !owned_skins.contains(&required.to_lowercase())
                        && !owned_skins.contains(&item_id.to_lowercase())
                    {
                        return None;
                    }
                    let item = index.resolve(ITEM_SKIN, item_id);
                    let delta = row
                        .get("ProgressionSchedule")
                        .and_then(|value| value.get("ProgressionDeltaPerLevel"))
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0);
                    let rewards = row
                        .get("RewardSchedule")
                        .and_then(|value| value.get("RewardsPerLevel"))
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0);
                    Some(json!({
                        "id": row.get("ID").cloned().unwrap_or(Value::Null),
                        "item": item,
                        "levels": delta,
                        "rewardLevels": rewards
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let value = json!({
        "available": true,
        "account": account,
        "activeContract": active,
        "weeklyRefill": data
            .get("MissionMetadata")
            .and_then(|value| value.get("WeeklyRefillTime"))
            .cloned()
            .unwrap_or(Value::Null),
        "contracts": contracts,
        "missions": missions,
        "recent": recent,
        "upgradeable": upgradeable,
        "upgradeCount": upgrades.as_array().map(Vec::len).unwrap_or(0)
    });
    state.put_json(cache_key, value.clone());
    Ok(value)
}

pub async fn activate(
    state: &Arc<AppState>,
    contract_id: &str,
    dry_run: bool,
) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let definitions = contract_map(&state.valo.list("contracts").await);
    let name = definitions
        .get(&contract_id.to_lowercase())
        .and_then(|value| value.get("displayName"))
        .and_then(Value::as_str)
        .unwrap_or("Contract");
    if dry_run || !settings.allow_live_actions {
        return Ok(json!({
            "ok": true,
            "status": "dry-run",
            "message": format!("DRY-RUN: would activate {name}.")
        }));
    }
    session
        .pd_post(
            &format!(
                "/contracts/v1/contracts/{}/special/{}",
                session.puuid, contract_id
            ),
            None,
        )
        .await
        .map_err(|error| format!("Couldn't activate the contract: {error}"))?;
    state
        .json_cache
        .lock()
        .remove(&format!("contracts:{}", session.puuid));
    Ok(json!({
        "ok": true,
        "status": "active",
        "message": format!("Activated {name}.")
    }))
}
