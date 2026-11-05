use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use parking_lot::{Mutex, RwLock};
use tauri::{AppHandle, Manager};

use crate::riot::RiotRuntime;
use crate::valo::Valo;

pub mod store;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub region: String,
    pub live_refresh_secs: u64,
    pub allow_live_actions: bool,
    pub discord_rpc: bool,
    pub favorite_crosshairs: Vec<u32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            region: String::new(),
            live_refresh_secs: 2,
            allow_live_actions: false,
            discord_rpc: true,
            favorite_crosshairs: Vec::new(),
        }
    }
}

impl Settings {
    pub fn path(dir: &PathBuf) -> PathBuf {
        dir.join("settings.json")
    }

    pub fn load(dir: &PathBuf) -> Self {
        std::fs::read_to_string(Self::path(dir))
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, dir: &PathBuf) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let raw = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::path(dir), raw).map_err(|e| e.to_string())
    }
}

fn relabel_rank_scale(data_dir: &PathBuf) {
    let mut history = store::read(data_dir, "history");
    if history.get("rankScale").and_then(serde_json::Value::as_u64) != Some(28) {
        history["rankScale"] = serde_json::json!(28);
        let _ = store::write(data_dir, "history", &history);
    }

    let mut encounters = store::read(data_dir, "encounters");
    if encounters.get("rankScale").and_then(serde_json::Value::as_u64) != Some(28) {
        if let Some(accounts) = encounters
            .get_mut("accounts")
            .and_then(serde_json::Value::as_object_mut)
        {
            for account in accounts.values_mut() {
                let Some(players) = account
                    .get_mut("players")
                    .and_then(serde_json::Value::as_object_mut)
                else {
                    continue;
                };
                for player in players.values_mut() {
                    let tier = player
                        .get("rankTier")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0);
                    if tier > 0 {
                        let rank = crate::catalog::rank_from_tier(tier as i32);
                        player["rank"] = serde_json::json!(rank.name);
                        player["rankColor"] = serde_json::json!(rank.color);
                    }
                    let peak = player
                        .get("peakTier")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0);
                    if peak > 0 {
                        let rank = crate::catalog::rank_from_tier(peak as i32);
                        player["peakRank"] = serde_json::json!(rank.name);
                    }
                }
            }
        }
        encounters["rankScale"] = serde_json::json!(28);
        let _ = store::write(data_dir, "encounters", &encounters);
    }
}

#[derive(Clone)]
pub struct CachedLive {
    pub at: Instant,
    pub value: serde_json::Value,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub http: reqwest::Client,
    pub settings: RwLock<Settings>,
    pub live_cache: Mutex<Option<CachedLive>>,
    pub json_cache: Mutex<HashMap<String, (Instant, serde_json::Value)>>,
    pub riot: RiotRuntime,
    pub valo: Arc<Valo>,
    pub stats: Mutex<HashMap<String, crate::live::PlayerStats>>,
    pub ranks: Mutex<HashMap<String, (crate::riot::RankInfo, Instant)>>,
    pub details: Mutex<HashMap<String, (serde_json::Value, Instant)>>,
    pub meta: Mutex<HashMap<String, serde_json::Value>>,
    pub instalock: Arc<crate::instalock::InstalockState>,
    pub soundboard: Arc<crate::soundboard::Soundboard>,
    pub offline: Arc<crate::offline_client::OfflineClient>,
    pub remote: Arc<crate::remote::RemoteState>,
}

impl AppState {
    pub fn new_headless(data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        relabel_rank_scale(&data_dir);
        let settings = Settings::load(&data_dir);
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .user_agent("Valotrack/0.1")
            .timeout(Duration::from_secs(12))
            .pool_max_idle_per_host(16)
            .build()
            .map_err(|e| e.to_string())?;
        let valo = Valo::new(http.clone());
        let offline = crate::offline_client::OfflineClient::new(&data_dir, http.clone());
        let stats = crate::live::load_stats(&data_dir);
        let soundboard = Arc::new(crate::soundboard::Soundboard::new(&data_dir));
        Ok(Self {
            data_dir,
            http: http.clone(),
            settings: RwLock::new(settings),
            live_cache: Mutex::new(None),
            json_cache: Mutex::new(HashMap::new()),
            riot: RiotRuntime::new(http),
            valo,
            stats: Mutex::new(stats),
            ranks: Mutex::new(HashMap::new()),
            details: Mutex::new(HashMap::new()),
            meta: Mutex::new(HashMap::new()),
            instalock: Arc::new(crate::instalock::InstalockState::new()),
            soundboard,
            offline,
            remote: Arc::new(crate::remote::RemoteState::new()),
        })
    }

    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("could not resolve app data directory: {e}"))?;
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        relabel_rank_scale(&data_dir);
        let settings = Settings::load(&data_dir);
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .user_agent("Valotrack/0.1")
            .timeout(Duration::from_secs(12))
            .pool_max_idle_per_host(16)
            .build()
            .map_err(|e| e.to_string())?;
        let valo = Valo::new(http.clone());
        let offline = crate::offline_client::OfflineClient::new(&data_dir, http.clone());
        let stats = crate::live::load_stats(&data_dir);
        let soundboard = Arc::new(crate::soundboard::Soundboard::new(&data_dir));
        Ok(Self {
            data_dir,
            http: http.clone(),
            settings: RwLock::new(settings),
            live_cache: Mutex::new(None),
            json_cache: Mutex::new(HashMap::new()),
            riot: RiotRuntime::new(http),
            valo,
            stats: Mutex::new(stats),
            ranks: Mutex::new(HashMap::new()),
            details: Mutex::new(HashMap::new()),
            meta: Mutex::new(HashMap::new()),
            instalock: Arc::new(crate::instalock::InstalockState::new()),
            soundboard,
            offline,
            remote: Arc::new(crate::remote::RemoteState::new()),
        })
    }

    pub fn cache_json_for(&self, key: &str, ttl: Duration) -> Option<serde_json::Value> {
        let cache = self.json_cache.lock();
        cache
            .get(key)
            .filter(|(at, _)| at.elapsed() < ttl)
            .map(|(_, value)| value.clone())
    }

    pub fn cache_json(&self, key: &str) -> Option<serde_json::Value> {
        self.cache_json_for(key, Duration::from_secs(60))
    }

    pub fn put_json(&self, key: String, value: serde_json::Value) {
        let mut cache = self.json_cache.lock();
        if cache.len() > 128 {
            cache.clear();
        }
        cache.insert(key, (Instant::now(), value));
    }

    pub fn settings_snapshot(&self) -> Settings {
        self.settings.read().clone()
    }
}
