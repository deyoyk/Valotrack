use std::{
    path::PathBuf,
    process::Command,
    sync::Arc,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::offline::{
    capture_path, helper_exe_path, helper_state_path, set_capture_path, DATA_DIR_FLAG, HELPER_FLAG,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HelperInfo {
    pub port: u16,
    pub token: String,
    pub pid: u32,
    #[serde(default)]
    pub version: String,
}

pub struct OfflineClient {
    data_dir: PathBuf,
    http: reqwest::Client,
    helper_path: PathBuf,
}

impl OfflineClient {
    pub fn new(data_dir: &PathBuf, http: reqwest::Client) -> Arc<Self> {
        set_capture_path(capture_path(data_dir));
        Arc::new(Self {
            data_dir: data_dir.clone(),
            http,
            helper_path: helper_state_path(data_dir),
        })
    }

    fn stopped(&self) -> Value {
        let status = std::fs::read_to_string(self.data_dir.join("offline").join("status.txt"))
            .ok()
            .map(|value| value.trim().to_lowercase())
            .filter(|value| ["online", "offline", "away", "mobile"].contains(&value.as_str()))
            .unwrap_or_else(|| "offline".into());
        json!({
            "running": false,
            "active": false,
            "connected": false,
            "status": status,
            "enabled": status != "online",
            "configPort": Value::Null,
            "chatPort": Value::Null,
            "message": "Relay is stopped."
        })
    }

    fn info(&self) -> Option<HelperInfo> {
        let raw = std::fs::read_to_string(&self.helper_path).ok()?;
        let info: HelperInfo = serde_json::from_str(&raw).ok()?;
        if !crate::riot::process_exists(info.pid) {
            let _ = std::fs::remove_file(&self.helper_path);
            return None;
        }
        Some(info)
    }

    async fn live_info(&self) -> Option<HelperInfo> {
        let info = self.info()?;
        if self
            .get(&info, "/status")
            .await
            .ok()
            .filter(|value| value.get("running").and_then(Value::as_bool) == Some(true))
            .is_some()
        {
            return Some(info);
        }
        let _ = std::fs::remove_file(&self.helper_path);
        None
    }

    async fn get(&self, info: &HelperInfo, path: &str) -> Result<Value, String> {
        let response = self
            .http
            .get(format!(
                "http://127.0.0.1:{}{}?t={}",
                info.port, path, info.token
            ))
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|error| error.to_string())?;
        response_json(response).await
    }

    async fn post(&self, info: &HelperInfo, path: &str, body: Value) -> Result<Value, String> {
        let response = self
            .http
            .post(format!(
                "http://127.0.0.1:{}{}?t={}",
                info.port, path, info.token
            ))
            .json(&body)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|error| error.to_string())?;
        response_json(response).await
    }

    pub async fn status(&self) -> Value {
        if let Some(info) = self.live_info().await {
            if let Ok(value) = self.get(&info, "/status").await {
                return value;
            }
        }
        self.stopped()
    }

    pub async fn start(&self) -> Result<Value, String> {
        let info = self.ensure_helper().await?;
        self.get(&info, "/status").await
    }

    pub async fn launch(&self, status: Option<&str>) -> Result<Value, String> {
        let info = self.ensure_helper().await?;
        self.post(&info, "/launch", json!({ "status": status }))
            .await
    }

    pub async fn set_status(&self, status: &str) -> Result<Value, String> {
        let info = self
            .live_info()
            .await
            .ok_or_else(|| "Offline relay isn't running.".to_string())?;
        self.post(&info, "/status", json!({ "status": status }))
            .await
    }

    pub async fn set_enabled(&self, enabled: bool) -> Result<Value, String> {
        let info = self
            .live_info()
            .await
            .ok_or_else(|| "Offline relay isn't running.".to_string())?;
        self.post(&info, "/enabled", json!({ "enabled": enabled }))
            .await
    }

    pub async fn stop(&self) -> Result<Value, String> {
        if let Some(info) = self.info() {
            self.shutdown(&info).await;
        }
        let _ = std::fs::remove_file(&self.helper_path);
        let _ = std::fs::remove_file(helper_exe_path(&self.data_dir));
        Ok(self.stopped())
    }

    async fn shutdown(&self, info: &HelperInfo) {
        let _ = self.post(info, "/shutdown", json!({})).await;
        for _ in 0..50 {
            if !crate::riot::process_exists(info.pid) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn ensure_helper(&self) -> Result<HelperInfo, String> {
        if let Some(info) = self.live_info().await {
            if info.version == env!("CARGO_PKG_VERSION") {
                return Ok(info);
            }
            self.shutdown(&info).await;
            if crate::riot::process_exists(info.pid) {
                return Err("An older offline relay is still running. Restart Windows and try again."
                    .into());
            }
        }
        let exe = std::env::current_exe().map_err(|error| error.to_string())?;
        let helper = helper_exe_path(&self.data_dir);
        if let Some(parent) = helper.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut copied = false;
        for _ in 0..30 {
            match std::fs::copy(&exe, &helper) {
                Ok(_) => {
                    copied = true;
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(error) => return Err(format!("Couldn't prepare the offline relay: {error}")),
            }
        }
        if !copied {
            return Err("Couldn't prepare the offline relay. Close any running relay and retry."
                .into());
        }
        let _ = std::fs::remove_file(&self.helper_path);
        spawn_helper(&helper, &self.data_dir)
            .map_err(|_| "Couldn't start the offline relay helper.".to_string())?;
        for _ in 0..100 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            if let Some(info) = self.live_info().await {
                return Ok(info);
            }
        }
        Err("The offline relay helper didn't become ready. Check the engine log.".into())
    }
}

fn spawn_helper(exe: &PathBuf, data_dir: &PathBuf) -> Result<(), std::io::Error> {
    let mut command = Command::new(exe);
    command
        .arg(HELPER_FLAG)
        .arg(DATA_DIR_FLAG)
        .arg(data_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000 | 0x00000008 | 0x01000000);
        if command.spawn().is_ok() {
            return Ok(());
        }
        let mut fallback = Command::new(exe);
        fallback
            .arg(HELPER_FLAG)
            .arg(DATA_DIR_FLAG)
            .arg(data_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(0x08000000 | 0x00000008);
        fallback.spawn()?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        command.spawn()?;
        Ok(())
    }
}

async fn response_json(response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    let text = response.text().await.map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(text);
    }
    serde_json::from_str(&text).map_err(|error| error.to_string())
}
