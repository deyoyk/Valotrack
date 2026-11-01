use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Map, Value};

use crate::{
    app::AppState,
    content::{self, ItemIndex, ITEM_BUDDY, ITEM_CARD, ITEM_SKIN, ITEM_SPRAY, ITEM_TITLE},
};

pub async fn snapshot(state: &Arc<AppState>) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let cache_key = format!("loadout:{}", session.puuid);
    if let Some(cached) = state.cache_json_for(&cache_key, Duration::from_secs(60)) {
        return Ok(cached);
    }
    let loadout = match session
        .pd_get(&format!(
            "/personalization/v3/players/{}/playerloadout",
            session.puuid
        ))
        .await
    {
        Ok(value) if value.get("Guns").is_some() => value,
        _ => session
            .pd_get(&format!(
                "/personalization/v2/players/{}/playerloadout",
                session.puuid
            ))
            .await
            .map_err(|error| format!("Loadout unavailable: {error}"))?,
    };
    let index = ItemIndex::build(&state.valo).await;
    let account = content::account_label(&session).await;
    let owned_skin_ids = content::owned_ids(&session, ITEM_SKIN).await;
    let mut owned_skins = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for id in owned_skin_ids {
        let item = index.resolve(ITEM_SKIN, &id);
        let Some(skin_id) = item.get("skinId").and_then(Value::as_str) else {
            continue;
        };
        if !seen.insert(skin_id.to_lowercase()) {
            continue;
        }
        owned_skins.push(json!({
            "skinId": skin_id,
            "levelId": item.get("levelId").cloned().unwrap_or(Value::Null),
            "chromaId": item.get("defaultChromaId").cloned().unwrap_or(Value::Null),
            "name": item.get("name").cloned().unwrap_or(Value::Null),
            "icon": item.get("icon").cloned().unwrap_or(Value::Null),
            "tier": item.get("tier").cloned().unwrap_or(Value::Null),
            "vp": item.get("vp").cloned().unwrap_or(Value::Null),
            "weapon": item.get("weapon").cloned().unwrap_or(Value::Null),
            "weaponId": item.get("weaponId").cloned().unwrap_or(Value::Null)
        }));
    }
    owned_skins.sort_by_key(|item| {
        (
            item.get("weapon")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase(),
            item.get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase(),
        )
    });

    let guns = loadout
        .get("Guns")
        .and_then(Value::as_array)
        .map(|guns| {
            guns.iter()
                .map(|gun| {
                    let weapon_id = gun.get("ID").and_then(Value::as_str).unwrap_or("");
                    let weapon = index.weapon(weapon_id);
                    let skin = gun
                        .get("SkinLevelID")
                        .and_then(Value::as_str)
                        .filter(|id| !id.is_empty())
                        .map(|id| index.resolve(ITEM_SKIN, id))
                        .or_else(|| {
                            gun.get("SkinID")
                                .and_then(Value::as_str)
                                .filter(|id| !id.is_empty())
                                .map(|id| index.resolve(ITEM_SKIN, id))
                        });
                    let charm = gun
                        .get("CharmLevelID")
                        .and_then(Value::as_str)
                        .filter(|id| !id.is_empty())
                        .map(|id| index.resolve(ITEM_BUDDY, id))
                        .or_else(|| {
                            gun.get("CharmID")
                                .and_then(Value::as_str)
                                .filter(|id| !id.is_empty())
                                .map(|id| index.resolve(ITEM_BUDDY, id))
                        });
                    json!({
                        "id": weapon_id,
                        "weapon": weapon,
                        "skin": skin,
                        "charm": charm,
                        "skinId": gun.get("SkinID").cloned().unwrap_or(Value::Null),
                        "skinLevelId": gun.get("SkinLevelID").cloned().unwrap_or(Value::Null),
                        "chromaId": gun.get("ChromaID").cloned().unwrap_or(Value::Null)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let sprays = if let Some(expressions) = loadout.get("ActiveExpressions").and_then(Value::as_array)
    {
        expressions
            .iter()
            .enumerate()
            .map(|(index_no, expression)| {
                let spray_id = expression
                    .get("AssetID")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                json!({
                    "slot": expression.get("TypeID").cloned().unwrap_or(Value::Null),
                    "slotNumber": index_no + 1,
                    "spray": index.resolve(ITEM_SPRAY, spray_id)
                })
            })
            .collect::<Vec<_>>()
    } else {
        loadout
            .get("Sprays")
            .and_then(Value::as_array)
            .map(|sprays| {
                sprays
                    .iter()
                    .enumerate()
                    .map(|(index_no, spray)| {
                        let spray_id = spray.get("SprayID").and_then(Value::as_str).unwrap_or("");
                        json!({
                            "slot": spray.get("EquipSlotID").cloned().unwrap_or(Value::Null),
                            "slotNumber": index_no + 1,
                            "spray": index.resolve(ITEM_SPRAY, spray_id)
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };

    let identity = loadout
        .get("Identity")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let card_id = identity
        .get("PlayerCardID")
        .and_then(Value::as_str)
        .unwrap_or("");
    let title_id = identity
        .get("PlayerTitleID")
        .and_then(Value::as_str)
        .unwrap_or("");
    let card = index.resolve(ITEM_CARD, card_id);
    let title = index.resolve(ITEM_TITLE, title_id);

    let value = json!({
        "available": true,
        "account": account,
        "guns": guns,
        "sprays": sprays,
        "identity": {
            "card": card,
            "title": title,
            "accountLevel": identity.get("AccountLevel").cloned().unwrap_or(json!(0)),
            "hideAccountLevel": identity.get("HideAccountLevel").cloned().unwrap_or(json!(false))
        },
        "incognito": loadout.get("Incognito").cloned().unwrap_or(json!(false)),
        "ownedSkins": owned_skins,
        "version": loadout.get("Version").cloned().unwrap_or(json!(0))
    });
    state.put_json(cache_key, value.clone());
    Ok(value)
}

pub async fn set_skin(
    state: &Arc<AppState>,
    weapon_id: &str,
    skin_id: &str,
    dry_run: bool,
) -> Result<Value, String> {
    let settings = state.settings_snapshot();
    let session = state
        .riot
        .session(&settings.region)
        .await
        .map_err(|error| error.to_string())?;
    let current = match session
        .pd_get(&format!(
            "/personalization/v3/players/{}/playerloadout",
            session.puuid
        ))
        .await
    {
        Ok(value) if value.get("Guns").is_some() => value,
        _ => session
            .pd_get(&format!(
                "/personalization/v2/players/{}/playerloadout",
                session.puuid
            ))
            .await
            .map_err(|error| format!("Loadout unavailable: {error}"))?,
    };
    let index = ItemIndex::build(&state.valo).await;
    let owned_ids = content::owned_ids(&session, ITEM_SKIN).await;
    let mut chosen: Option<Value> = None;
    for id in &owned_ids {
        let item = index.resolve(ITEM_SKIN, id);
        if item
            .get("skinId")
            .and_then(Value::as_str)
            .map(|value| value.eq_ignore_ascii_case(skin_id))
            .unwrap_or(false)
            && item
                .get("weaponId")
                .and_then(Value::as_str)
                .map(|value| value.eq_ignore_ascii_case(weapon_id))
                .unwrap_or(false)
        {
            chosen = Some(item);
            break;
        }
    }
    let Some(chosen) = chosen else {
        return Err("That skin is not owned for this weapon.".into());
    };
    let skin_name = chosen
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Skin");
    let weapon_name = index
        .weapon(weapon_id)
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Weapon")
        .to_string();
    if dry_run || !settings.allow_live_actions {
        return Ok(json!({
            "ok": true,
            "status": "dry-run",
            "message": format!("DRY-RUN: would equip {skin_name} on {weapon_name}."),
            "skin": chosen
        }));
    }

    let mut body = current.clone();
    let Some(guns) = body.get_mut("Guns").and_then(Value::as_array_mut) else {
        return Err("The Riot client did not return a loadout.".into());
    };
    let Some(gun) = guns.iter_mut().find(|gun| {
        gun.get("ID")
            .and_then(Value::as_str)
            .map(|value| value.eq_ignore_ascii_case(weapon_id))
            .unwrap_or(false)
    }) else {
        return Err("That weapon is not in your loadout.".into());
    };
    let level_id = chosen
        .get("levelId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "That skin has no equippable level.".to_string())?;
    let chroma_id = chosen
        .get("defaultChromaId")
        .or_else(|| chosen.get("chromaId"))
        .and_then(Value::as_str)
        .unwrap_or("");
    gun["SkinID"] = json!(skin_id);
    gun["SkinLevelID"] = json!(level_id);
    gun["ChromaID"] = json!(chroma_id);

    let mut v3_payload = Map::new();
    for key in ["Guns", "ActiveExpressions", "DynamicOptions", "Identity", "Incognito"] {
        if let Some(value) = body.get(key) {
            v3_payload.insert(key.to_string(), value.clone());
        }
    }
    let mut v2_payload = Map::new();
    for key in ["Guns", "Sprays", "Identity", "Incognito"] {
        if let Some(value) = body.get(key) {
            v2_payload.insert(key.to_string(), value.clone());
        }
    }
    let response = match session
        .pd_put(
            &format!(
                "/personalization/v3/players/{}/playerloadout",
                session.puuid
            ),
            &Value::Object(v3_payload),
        )
        .await
    {
        Ok(value) if value.get("Guns").is_some() => value,
        _ => session
            .pd_put(
                &format!(
                    "/personalization/v2/players/{}/playerloadout",
                    session.puuid
                ),
                &Value::Object(v2_payload),
            )
            .await
            .map_err(|error| format!("Couldn't save the loadout: {error}"))?,
    };
    state
        .json_cache
        .lock()
        .remove(&format!("loadout:{}", session.puuid));
    Ok(json!({
        "ok": true,
        "status": "equipped",
        "message": format!("Equipped {skin_name} on {weapon_name}."),
        "skin": chosen,
        "loadout": response
    }))
}
