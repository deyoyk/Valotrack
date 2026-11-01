use std::collections::HashMap;

use serde_json::{json, Value};

use crate::{catalog, riot::Session, valo::Valo};

pub const ITEM_SKIN: &str = "e7c63390-eda7-46e0-bb7a-a6abdacd2433";
pub const ITEM_BUDDY: &str = "dd3bf334-87f3-40bd-b043-682a57a8dc3a";
pub const ITEM_CARD: &str = "3f296c07-64c3-494c-923b-fe692a4fa1bd";
pub const ITEM_SPRAY: &str = "d5f120f8-ff8c-4aac-92ea-f2b5acbe9475";
pub const ITEM_AGENT: &str = "01bb38e1-da47-4e6a-9b3d-945fe4655707";
pub const ITEM_TITLE: &str = "title";
pub const CURRENCY_VP: &str = "85ad13f7-3d1b-5128-9eb2-7cd8ee0b5741";
pub const CURRENCY_RAD: &str = "e59aa87c-4cbf-517a-5983-6e81511be9b7";
pub const CURRENCY_KC: &str = "85ca954a-41f2-ce94-9b45-8ca3dd39a00d";

const TIERS: &[(&str, i32, &str)] = &[
    ("12683d76-48d7-84a3-4e09-6985794f0445", 875, "Select"),
    ("0cebb8be-46d7-c12a-d306-e9907bfc5a25", 1275, "Deluxe"),
    ("60bca009-4182-7998-dee7-b8a2558dc369", 1775, "Premium"),
    ("e046854e-406c-37f4-6607-19a9ba8426fc", 2175, "Exclusive"),
    ("411e4a55-4e59-7757-41f0-86a53f101bb5", 2475, "Ultra"),
];
const MELEE_VP: i32 = 3550;

pub fn key(item_type: &str, id: &str) -> String {
    format!("{}:{}", item_type.to_lowercase(), id.to_lowercase())
}

pub fn type_label(item_type: &str) -> &'static str {
    match item_type.to_lowercase().as_str() {
        ITEM_SKIN => "skin",
        ITEM_BUDDY => "buddy",
        ITEM_CARD => "card",
        ITEM_SPRAY => "spray",
        ITEM_AGENT => "agent",
        ITEM_TITLE => "title",
        _ => "item",
    }
}

pub fn vp_cost(cost: &Value) -> i64 {
    cost.get(CURRENCY_VP)
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

pub fn currency_cost(cost: &Value) -> (i64, &'static str) {
    for (id, currency) in [
        (CURRENCY_VP, "vp"),
        (CURRENCY_KC, "kc"),
        (CURRENCY_RAD, "rad"),
    ] {
        if let Some(value) = cost.get(id).and_then(Value::as_i64) {
            return (value, currency);
        }
    }
    (0, "vp")
}

#[derive(Default)]
pub struct ItemIndex {
    items: HashMap<String, Value>,
    bundles: HashMap<String, Value>,
    weapons: HashMap<String, Value>,
}

impl ItemIndex {
    pub async fn build(valo: &Valo) -> Self {
        let mut index = Self::default();
        for weapon in valo.list("weapons").await {
            let Some(weapon_id) = weapon.get("uuid").and_then(Value::as_str) else {
                continue;
            };
            let weapon_name = weapon
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or("Weapon");
            let weapon_icon = weapon.get("displayIcon").and_then(Value::as_str);
            index.weapons.insert(
                weapon_id.to_lowercase(),
                json!({
                    "id": weapon_id,
                    "name": weapon_name,
                    "icon": weapon_icon
                }),
            );
            let melee = weapon_name.eq_ignore_ascii_case("melee");
            let Some(skins) = weapon.get("skins").and_then(Value::as_array) else {
                continue;
            };
            for skin in skins {
                let Some(skin_id) = skin.get("uuid").and_then(Value::as_str) else {
                    continue;
                };
                let name = skin
                    .get("displayName")
                    .and_then(Value::as_str)
                    .unwrap_or("Skin")
                    .trim();
                let tier_uuid = skin
                    .get("contentTierUuid")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_lowercase();
                let (skin_vp, tier) = TIERS
                    .iter()
                    .find(|(id, _, _)| *id == tier_uuid)
                    .map(|(_, vp, tier)| (*vp, *tier))
                    .unwrap_or((0, "Standard"));
                let Some(levels) = skin.get("levels").and_then(Value::as_array) else {
                    continue;
                };
                let base_icon = skin.get("displayIcon").and_then(Value::as_str);
                let meta = json!({
                    "type": "skin",
                    "uuid": skin_id,
                    "skinId": skin_id,
                    "name": name,
                    "icon": base_icon,
                    "tier": if melee && skin_vp > 0 { "Exclusive" } else { tier },
                    "vp": if melee && skin_vp > 0 { MELEE_VP } else { skin_vp },
                    "weapon": weapon_name,
                    "weaponId": weapon_id
                });
                index.items.insert(key(ITEM_SKIN, skin_id), meta.clone());
                for level in levels {
                    let Some(level_id) = level.get("uuid").and_then(Value::as_str) else {
                        continue;
                    };
                    let icon = level
                        .get("displayIcon")
                        .and_then(Value::as_str)
                        .or(base_icon);
                    let default_chroma = level
                        .get("chromas")
                        .and_then(Value::as_array)
                        .and_then(|chromas| chromas.first())
                        .and_then(|chroma| chroma.get("uuid"))
                        .cloned()
                        .unwrap_or(Value::Null);
                    let mut level_meta = meta.clone();
                    level_meta["uuid"] = json!(level_id);
                    level_meta["levelId"] = json!(level_id);
                    level_meta["defaultChromaId"] = default_chroma;
                    level_meta["icon"] = json!(icon);
                    index
                        .items
                        .insert(key(ITEM_SKIN, level_id), level_meta.clone());
                    if let Some(chromas) = level.get("chromas").and_then(Value::as_array) {
                        for chroma in chromas {
                            let Some(chroma_id) = chroma.get("uuid").and_then(Value::as_str) else {
                                continue;
                            };
                            let chroma_icon = chroma
                                .get("fullRender")
                                .and_then(Value::as_str)
                                .or_else(|| chroma.get("displayIcon").and_then(Value::as_str))
                                .or(icon);
                            let mut chroma_meta = level_meta.clone();
                            chroma_meta["uuid"] = json!(chroma_id);
                            chroma_meta["chromaId"] = json!(chroma_id);
                            chroma_meta["icon"] = json!(chroma_icon);
                            chroma_meta["chroma"] = chroma
                                .get("displayName")
                                .cloned()
                                .unwrap_or(Value::Null);
                            index
                                .items
                                .insert(key(ITEM_SKIN, chroma_id), chroma_meta);
                        }
                    }
                }
            }
        }

        for row in valo.list("buddies").await {
            let Some(name) = row.get("displayName").and_then(Value::as_str) else {
                continue;
            };
            let base_icon = row.get("displayIcon").and_then(Value::as_str);
            if let Some(uuid) = row.get("uuid").and_then(Value::as_str) {
                index.items.insert(
                    key(ITEM_BUDDY, uuid),
                    json!({ "type": "buddy", "uuid": uuid, "name": name, "icon": base_icon }),
                );
            }
            if let Some(levels) = row.get("levels").and_then(Value::as_array) {
                for level in levels {
                    let Some(uuid) = level.get("uuid").and_then(Value::as_str) else {
                        continue;
                    };
                    let icon = level
                        .get("displayIcon")
                        .and_then(Value::as_str)
                        .or(base_icon);
                    index.items.insert(
                        key(ITEM_BUDDY, uuid),
                        json!({ "type": "buddy", "uuid": uuid, "name": name, "icon": icon }),
                    );
                }
            }
        }

        for (item_type, path, icon_keys) in [
            (ITEM_CARD, "playercards", &["displayIcon", "smallArt", "wideArt"][..]),
            (ITEM_SPRAY, "sprays", &["displayIcon", "fullIcon"][..]),
        ] {
            for row in valo.list(path).await {
                let Some(uuid) = row.get("uuid").and_then(Value::as_str) else {
                    continue;
                };
                let Some(name) = row.get("displayName").and_then(Value::as_str) else {
                    continue;
                };
                let icon = icon_keys
                    .iter()
                    .find_map(|key| row.get(*key).and_then(Value::as_str));
                index.items.insert(
                    key(item_type, uuid),
                    json!({
                        "type": type_label(item_type),
                        "uuid": uuid,
                        "name": name,
                        "icon": icon
                    }),
                );
            }
        }

        for agent in catalog::agents() {
            index.items.insert(
                key(ITEM_AGENT, agent.uuid),
                json!({
                    "type": "agent",
                    "uuid": agent.uuid,
                    "name": agent.name,
                    "icon": agent.portrait
                }),
            );
        }

        for (uuid, title) in valo.titles().await {
            index.items.insert(
                key(ITEM_TITLE, &uuid),
                json!({ "type": "title", "uuid": uuid, "name": title, "icon": null }),
            );
        }

        for bundle in valo.list("bundles").await {
            let Some(uuid) = bundle.get("uuid").and_then(Value::as_str) else {
                continue;
            };
            index.bundles.insert(
                uuid.to_lowercase(),
                json!({
                    "id": uuid,
                    "name": bundle.get("displayName").and_then(Value::as_str).unwrap_or("Bundle"),
                    "icon": bundle.get("displayIcon").and_then(Value::as_str)
                }),
            );
        }
        index
    }

    pub fn resolve(&self, item_type: &str, item_id: &str) -> Value {
        let item_type = item_type.to_lowercase();
        let currency = match item_type.as_str() {
            CURRENCY_VP => Some(("VALORANT Points", "vp")),
            CURRENCY_RAD => Some(("Radianite", "rad")),
            CURRENCY_KC => Some(("Kingdom Credits", "kc")),
            _ => None,
        };
        if let Some((name, code)) = currency {
            return json!({
                "type": "currency",
                "currency": code,
                "uuid": item_id,
                "name": name,
                "icon": format!(
                    "https://media.valorant-api.com/currencies/{}/displayicon.png",
                    item_id
                )
            });
        }
        let mut value = self
            .items
            .get(&key(&item_type, item_id))
            .cloned()
            .unwrap_or_else(|| {
                json!({
                    "type": type_label(&item_type),
                    "uuid": item_id,
                    "name": "Item",
                    "icon": null
                })
            });
        value["uuid"] = json!(item_id);
        value
    }

    pub fn weapon(&self, id: &str) -> Value {
        self.weapons.get(&id.to_lowercase()).cloned().unwrap_or_else(|| {
            json!({ "id": id, "name": id, "icon": null })
        })
    }

    pub fn bundle(&self, id: &str) -> Value {
        self.bundles.get(&id.to_lowercase()).cloned().unwrap_or_else(|| {
            json!({ "id": id, "name": "Bundle", "icon": null })
        })
    }
}

pub async fn owned_ids(session: &Session, item_type: &str) -> Vec<String> {
    session
        .pd_get(&format!(
            "/store/v1/entitlements/{}/{}",
            session.puuid, item_type
        ))
        .await
        .ok()
        .and_then(|value| value.get("Entitlements").cloned())
        .and_then(|value| value.as_array().cloned())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("ItemID").and_then(Value::as_str))
                .map(str::to_lowercase)
                .collect()
        })
        .unwrap_or_default()
}

pub async fn account_label(session: &Session) -> String {
    session
        .local_get("/player-account/aliases/v1/active")
        .await
        .ok()
        .and_then(|value| {
            let name = value.get("game_name").and_then(Value::as_str).unwrap_or("");
            let tag = value.get("tag_line").and_then(Value::as_str).unwrap_or("");
            if name.is_empty() {
                None
            } else if tag.is_empty() {
                Some(name.to_string())
            } else {
                Some(format!("{name}#{tag}"))
            }
        })
        .unwrap_or_else(|| session.puuid.clone())
}

pub async fn owned_set(session: &Session, index: &ItemIndex) -> std::collections::HashSet<String> {
    let mut owned = std::collections::HashSet::new();
    for item_type in [ITEM_SKIN, ITEM_BUDDY, ITEM_CARD, ITEM_SPRAY, ITEM_AGENT] {
        for id in owned_ids(session, item_type).await {
            owned.insert(key(item_type, &id));
            let resolved = index.resolve(item_type, &id);
            if let Some(skin_id) = resolved.get("skinId").and_then(Value::as_str) {
                owned.insert(key(item_type, skin_id));
            }
        }
    }
    owned
}

pub fn is_owned(index: &ItemIndex, owned: &std::collections::HashSet<String>, item_type: &str, item_id: &str) -> bool {
    if owned.contains(&key(item_type, item_id)) {
        return true;
    }
    let resolved = index.resolve(item_type, item_id);
    resolved
        .get("skinId")
        .and_then(Value::as_str)
        .map(|skin_id| owned.contains(&key(item_type, skin_id)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_keys_are_case_insensitive() {
        assert_eq!(key(ITEM_SKIN, "ABC"), key(ITEM_SKIN, "abc"));
    }

    #[test]
    fn currency_cost_reads_each_wallet() {
        assert_eq!(
            currency_cost(&json!({ CURRENCY_VP: 2175 })),
            (2175, "vp")
        );
        assert_eq!(
            currency_cost(&json!({ CURRENCY_KC: 5500 })),
            (5500, "kc")
        );
        assert_eq!(
            currency_cost(&json!({ CURRENCY_RAD: 40 })),
            (40, "rad")
        );
        assert_eq!(currency_cost(&json!({})), (0, "vp"));
    }

    #[test]
    fn unknown_items_keep_a_useful_fallback() {
        let index = ItemIndex::default();
        let item = index.resolve(ITEM_SKIN, "missing");
        assert_eq!(item["type"], "skin");
        assert_eq!(item["name"], "Item");
    }
}
