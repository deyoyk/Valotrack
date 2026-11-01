use serde::Serialize;
use tauri::State;

use crate::{app::AppState, riot};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub ok: bool,
    pub app_version: &'static str,
    pub client_available: bool,
    pub region: Option<String>,
    pub lockfile_path: String,
}

#[tauri::command]
pub fn health() -> Health {
    let status = riot::client_status();
    Health {
        ok: true,
        app_version: env!("CARGO_PKG_VERSION"),
        client_available: status.available,
        region: status.region,
        lockfile_path: status.lockfile_path,
    }
}

#[tauri::command]
pub async fn client_status() -> serde_json::Value {
    tokio::task::spawn_blocking(|| serde_json::to_value(riot::client_status()).unwrap_or_default())
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub async fn live_board(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    crate::live::board(&state).await
}

#[tauri::command]
pub async fn queue_state() -> Result<serde_json::Value, String> {
    riot::party_state().await
}

#[tauri::command]
pub fn agents() -> serde_json::Value {
    crate::catalog::agents_json()
}

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> crate::app::Settings {
    state
        .settings
        .read()
        .map(|s| s.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub fn settings_set(
    state: State<'_, AppState>,
    settings: crate::app::Settings,
) -> Result<crate::app::Settings, String> {
    settings.save(&state.data_dir)?;
    let mut lock = state.settings.write().map_err(|e| e.to_string())?;
    *lock = settings.clone();
    Ok(settings)
}
