use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use tokio::sync::RwLock;

const BASE: &str = "https://valorant-api.com/v1";
const SKIN_SOCKET: &str = "bcef87d6-209b-46c6-8b19-fbe40bd95abc";
const TIER_VP: &[(&str, i32, &str)] = &[
    ("12683d76-48d7-84a3-4e09-6985794f0445", 875, "Select"),
    ("0cebb8be-46d7-c12a-d306-e9907bfc5a25", 1275, "Deluxe"),
    ("60bca009-4182-7998-dee7-b8a2558dc369", 1775, "Premium"),
    ("e046854e-406c-37f4-6607-19a9ba8426fc", 2175, "Exclusive"),
    ("411e4a55-4e59-7757-41f0-86a53f101bb5", 2475, "Ultra"),
];
const MELEE_VP: i32 = 3550;
const WEAPON_ORDER: &[&str] = &[
    "Vandal",
    "Phantom",
    "Operator",
    "Sheriff",
    "Classic",
    "Ghost",
    "Frenzy",
    "Spectre",
    "Stinger",
    "Bulldog",
    "Guardian",
    "Marshal",
    "Outlaw",
    "Bucky",
    "Judge",
    "Ares",
    "Odin",
    "Shorty",
    "Melee",
];

pub struct Valo {
    http: reqwest::Client,
    cache: RwLock<HashMap<String, (Instant, Value)>>,
}

fn act_number(name: &str) -> Option<u32> {
    let token = name.trim().split_whitespace().last()?;
    if let Ok(number) = token.parse::<u32>() {
        return Some(number);
    }
    let mut total = 0u32;
    let mut previous = 0u32;
    for ch in token.to_uppercase().chars().rev() {
        let value = match ch {
            'I' => 1,
            'V' => 5,
            'X' => 10,
            'L' => 50,
            'C' => 100,
            _ => return None,
        };
        if value < previous {
            total = total.saturating_sub(value);
        } else {
            total += value;
            previous = value;
        }
    }
    (total > 0).then_some(total)
}

fn episode_label(name: &str) -> Option<String> {
    for token in name.split_whitespace() {
        let has_alpha = token.chars().any(|c| c.is_alphabetic());
        let has_digit = token.chars().any(|c| c.is_ascii_digit());
        if has_alpha && has_digit {
            return Some(token.to_uppercase());
        }
    }
    if name.trim().to_uppercase().starts_with("EPISODE") {
        return act_number(name).map(|n| format!("E{n}"));
    }
    None
}

impl Valo {
    pub fn new(http: reqwest::Client) -> Arc<Self> {
        Arc::new(Self {
            http,
            cache: RwLock::new(HashMap::new()),
        })
    }

    async fn raw(&self, path: &str) -> Option<Value> {
        if let Some((at, value)) = self.cache.read().await.get(path) {
            if at.elapsed() < Duration::from_secs(6 * 3600) {
                return Some(value.clone());
            }
        }
        let response = self
            .http
            .get(format!("{BASE}/{path}"))
            .timeout(Duration::from_secs(12))
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        let payload: Value = response.json().await.ok()?;
        let value = payload.get("data")?.clone();
        self.cache.write().await.insert(
            path.to_string(),
            (Instant::now(), value.clone()),
        );
        Some(value)
    }

    pub async fn list(&self, path: &str) -> Vec<Value> {
        self.raw(path)
            .await
            .and_then(|value| value.as_array().cloned())
            .unwrap_or_default()
    }

    pub async fn rank_icons(&self) -> HashMap<i32, (Option<String>, Option<String>)> {
        let mut out = HashMap::new();
        let Some(sets) = self.raw("competitivetiers").await else {
            return out;
        };
        let Some(set) = sets.as_array().and_then(|sets| sets.last()) else {
            return out;
        };
        if let Some(tiers) = set.get("tiers").and_then(Value::as_array) {
            for tier in tiers {
                let id = tier.get("tier").and_then(Value::as_i64).unwrap_or(0) as i32;
                out.insert(
                    id,
                    (
                        tier.get("smallIcon")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        tier.get("largeIcon")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    ),
                );
            }
        }
        out
    }

    pub async fn rank_icon(&self, tier: i32) -> Option<String> {
        self.rank_icons()
            .await
            .get(&tier.clamp(0, 27))
            .and_then(|(small, _)| small.clone())
            .or_else(|| {
                Some(format!(
                    "https://media.valorant-api.com/competitivetiers/03621f52-342b-cf4e-4f86-9350a49c6d04/{}/smallicon.png",
                    tier.clamp(0, 27)
                ))
            })
    }

    pub async fn titles(&self) -> HashMap<String, String> {
        let mut out = HashMap::new();
        if let Some(rows) = self.raw("playertitles").await.and_then(|v| v.as_array().cloned()) {
            for row in rows {
                if let (Some(uuid), Some(text)) = (
                    row.get("uuid").and_then(Value::as_str),
                    row.get("titleText").and_then(Value::as_str),
                ) {
                    out.insert(uuid.to_lowercase(), text.to_string());
                }
            }
        }
        out
    }

    pub async fn maps(&self) -> Vec<Value> {
        self.raw("maps")
            .await
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
    }

    pub async fn season_label(&self, season_id: &str) -> Option<String> {
        if season_id.is_empty() {
            return None;
        }
        let seasons = self.raw("seasons").await?;
        let rows = seasons.as_array()?;
        let by_id: HashMap<String, &Value> = rows
            .iter()
            .filter_map(|row| {
                Some((
                    row.get("uuid")?.as_str()?.to_lowercase(),
                    row,
                ))
            })
            .collect();
        let act = by_id.get(&season_id.to_lowercase())?;
        let act_name = act.get("displayName").and_then(Value::as_str).unwrap_or("");
        let number = act_number(act_name)?;
        let parent_id = act
            .get("parentUuid")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let episode = by_id
            .get(&parent_id)
            .and_then(|row| row.get("displayName"))
            .and_then(Value::as_str)
            .and_then(episode_label);
        Some(match episode {
            Some(label) => format!("{label} Act {number}"),
            None => format!("Act {number}"),
        })
    }

    pub async fn map_splash(&self, name: &str) -> Option<String> {
        if name.is_empty() || name == "Unknown" {
            return None;
        }
        self.maps()
            .await
            .into_iter()
            .find(|map| map.get("displayName").and_then(Value::as_str) == Some(name))
            .and_then(|map| {
                map.get("splash")
                    .or_else(|| map.get("listViewIcon"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
    }

    pub async fn weapons(&self) -> Vec<Value> {
        self.raw("weapons")
            .await
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
    }

    pub async fn skins(&self) -> Vec<Value> {
        self.raw("weapons/skins")
            .await
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
    }

    pub async fn skin_by_id(&self, skin_id: &str, weapon: &str) -> Option<Value> {
        let target = skin_id.to_lowercase();
        let skins = self.skins().await;
        let skin = skins.into_iter().find(|skin| {
            skin.get("uuid")
                .and_then(Value::as_str)
                .map(|uuid| uuid.eq_ignore_ascii_case(&target))
                .unwrap_or(false)
        })?;
        let name = skin
            .get("displayName")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .trim_end_matches(&format!(" {weapon}"))
            .trim()
            .to_string();
        Some(json!({
            "name": if name.is_empty() { weapon } else { &name },
            "icon": skin.get("displayIcon").and_then(Value::as_str)
        }))
    }

    pub async fn weapon_icon(&self, weapon_uuid: &str) -> Option<String> {
        self.weapons()
            .await
            .into_iter()
            .find(|weapon| {
                weapon
                    .get("uuid")
                    .and_then(Value::as_str)
                    .map(|uuid| uuid.eq_ignore_ascii_case(weapon_uuid))
                    .unwrap_or(false)
            })
            .and_then(|weapon| {
                weapon
                    .get("displayIcon")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
    }

    pub async fn loadout_weapons(&self, items: &serde_json::Map<String, Value>) -> Vec<Value> {
        let weapons = self.weapons().await;
        let names: HashMap<String, (String, Option<String>)> = weapons
            .iter()
            .filter_map(|weapon| {
                Some((
                    weapon.get("uuid")?.as_str()?.to_lowercase(),
                    (
                        weapon.get("displayName")?.as_str()?.to_string(),
                        weapon
                            .get("displayIcon")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    ),
                ))
            })
            .collect();
        let skins = self.skins().await;
        let skin_map: HashMap<String, (String, Option<String>)> = skins
            .iter()
            .filter_map(|skin| {
                Some((
                    skin.get("uuid")?.as_str()?.to_lowercase(),
                    (
                        skin.get("displayName")?.as_str()?.trim().to_string(),
                        skin
                            .get("displayIcon")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    ),
                ))
            })
            .collect();
        let mut rows = Vec::new();
        for (weapon_uuid, item) in items {
            let Some((weapon_name, weapon_icon)) = names.get(&weapon_uuid.to_lowercase()) else {
                continue;
            };
            let skin_id = item
                .get("Sockets")
                .and_then(|sockets| sockets.get(SKIN_SOCKET))
                .and_then(|socket| socket.get("Item"))
                .and_then(|item| item.get("ID"))
                .and_then(Value::as_str);
            let mut skin = skin_id
                .and_then(|id| skin_map.get(&id.to_lowercase()))
                .map(|(name, icon)| {
                    json!({
                        "name": name.replace(&format!(" {weapon_name}"), ""),
                        "icon": icon
                    })
                });
            if skin
                .as_ref()
                .and_then(|s| s.get("name"))
                .and_then(Value::as_str)
                .map(|name| name.eq_ignore_ascii_case("standard") || name.is_empty())
                .unwrap_or(true)
            {
                skin = Some(json!({ "name": "Standard", "icon": weapon_icon }));
            }
            let order = WEAPON_ORDER
                .iter()
                .position(|name| name == weapon_name)
                .unwrap_or(99);
            rows.push((
                order,
                json!({ "weapon": weapon_name, "skin": skin }),
            ));
        }
        rows.sort_by_key(|(order, _)| *order);
        rows.into_iter().map(|(_, row)| row).collect()
    }

    pub async fn base_skin_levels(&self) -> HashMap<String, Value> {
        let mut out = HashMap::new();
        let weapons = self.weapons().await;
        for weapon in weapons {
            let melee = weapon
                .get("displayName")
                .and_then(Value::as_str)
                .map(|name| name.eq_ignore_ascii_case("melee"))
                .unwrap_or(false);
            let Some(skins) = weapon.get("skins").and_then(Value::as_array) else {
                continue;
            };
            for skin in skins {
                let tier_uuid = skin
                    .get("contentTierUuid")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_lowercase();
                let Some((_, vp, tier_name)) =
                    TIER_VP.iter().find(|(id, _, _)| *id == tier_uuid)
                else {
                    continue;
                };
                let Some(levels) = skin.get("levels").and_then(Value::as_array) else {
                    continue;
                };
                let Some(base) = levels.first() else {
                    continue;
                };
                let Some(level_id) = base.get("uuid").and_then(Value::as_str) else {
                    continue;
                };
                out.insert(
                    level_id.to_lowercase(),
                    json!({
                        "name": skin.get("displayName").and_then(Value::as_str).unwrap_or("").trim(),
                        "icon": base.get("displayIcon").and_then(Value::as_str).or_else(|| skin.get("displayIcon").and_then(Value::as_str)),
                        "vp": if melee { MELEE_VP } else { *vp },
                        "tier": if melee { "Exclusive" } else { tier_name }
                    }),
                );
            }
        }
        out
    }

    pub async fn contract_rewards(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(contracts) = self.raw("contracts").await.and_then(|v| v.as_array().cloned()) {
            for contract in contracts {
                let Some(chapters) = contract
                    .get("content")
                    .and_then(|c| c.get("chapters"))
                    .and_then(Value::as_array)
                else {
                    continue;
                };
                for chapter in chapters {
                    let mut rewards = chapter
                        .get("freeRewards")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    if let Some(levels) = chapter.get("levels").and_then(Value::as_array) {
                        rewards.extend(levels.iter().filter_map(|level| level.get("reward").cloned()));
                    }
                    for reward in rewards {
                        if reward.get("type").and_then(Value::as_str)
                            == Some("EquippableSkinLevel")
                        {
                            if let Some(uuid) = reward.get("uuid").and_then(Value::as_str) {
                                out.push(uuid.to_lowercase());
                            }
                        }
                    }
                }
            }
        }
        out
    }

    async fn item_meta(&self, path: &str) -> HashMap<String, Value> {
        let mut out = HashMap::new();
        let Some(rows) = self.raw(path).await.and_then(|value| value.as_array().cloned()) else {
            return out;
        };
        for row in rows {
            let Some(uuid) = row.get("uuid").and_then(Value::as_str) else {
                continue;
            };
            let Some(name) = row.get("displayName").and_then(Value::as_str) else {
                continue;
            };
            let icon = row
                .get("displayIcon")
                .or_else(|| row.get("fullIcon"))
                .or_else(|| row.get("smallArt"))
                .or_else(|| row.get("wideArt"))
                .and_then(Value::as_str)
                .map(str::to_string);
            out.insert(
                uuid.to_lowercase(),
                json!({ "name": name, "icon": icon }),
            );
        }
        out
    }

    pub async fn buddy_meta(&self) -> HashMap<String, Value> {
        let mut out = HashMap::new();
        let Some(rows) = self.raw("buddies").await.and_then(|value| value.as_array().cloned()) else {
            return out;
        };
        for row in rows {
            let Some(name) = row.get("displayName").and_then(Value::as_str) else {
                continue;
            };
            let base_icon = row.get("displayIcon").and_then(Value::as_str);
            if let Some(levels) = row.get("levels").and_then(Value::as_array) {
                for level in levels {
                    let Some(uuid) = level.get("uuid").and_then(Value::as_str) else {
                        continue;
                    };
                    let icon = level
                        .get("displayIcon")
                        .and_then(Value::as_str)
                        .or(base_icon)
                        .map(str::to_string);
                    out.insert(
                        uuid.to_lowercase(),
                        json!({ "name": name, "icon": icon }),
                    );
                }
            }
            if let Some(uuid) = row.get("uuid").and_then(Value::as_str) {
                out.insert(
                    uuid.to_lowercase(),
                    json!({ "name": name, "icon": base_icon }),
                );
            }
        }
        out
    }

    pub async fn card_meta(&self) -> HashMap<String, Value> {
        self.item_meta("playercards").await
    }

    pub async fn spray_meta(&self) -> HashMap<String, Value> {
        self.item_meta("sprays").await
    }
}
