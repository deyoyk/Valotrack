use std::sync::Arc;

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
pub fn client_status() -> serde_json::Value {
    serde_json::to_value(riot::client_status()).unwrap_or_default()
}

#[tauri::command]
pub async fn live_board(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    crate::live::board(state.inner()).await
}

#[tauri::command]
pub async fn queue_state(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    crate::live::party_state(state.inner()).await
}

#[tauri::command]
pub fn agents() -> serde_json::Value {
    crate::catalog::agents_json()
}

#[tauri::command]
pub fn settings_get(state: State<'_, Arc<AppState>>) -> crate::app::Settings {
    state.settings_snapshot()
}

#[tauri::command]
pub fn settings_set(
    state: State<'_, Arc<AppState>>,
    settings: crate::app::Settings,
) -> Result<crate::app::Settings, String> {
    settings.save(&state.data_dir)?;
    *state.settings.write() = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub async fn queue_action(
    state: State<'_, Arc<AppState>>,
    action: String,
    queue_id: Option<String>,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    Ok(crate::actions::queue_action(state.inner(), &action, queue_id, dry_run).await)
}

#[tauri::command]
pub async fn dodge(
    state: State<'_, Arc<AppState>>,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    Ok(crate::actions::dodge(state.inner(), dry_run).await)
}

#[tauri::command]
pub async fn check_side(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    Ok(crate::actions::check_side(state.inner()).await)
}

#[tauri::command]
pub async fn instalock_now(
    state: State<'_, Arc<AppState>>,
    agent: String,
    mode: String,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    Ok(crate::actions::live_instalock(state.inner(), &agent, &mode, dry_run).await)
}

#[tauri::command]
pub fn instalock_start(
    state: State<'_, Arc<AppState>>,
    agent: String,
    mode: String,
    delay: f64,
    per_map: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if state.settings_snapshot().allow_live_actions {
        state.instalock.start(
            state.inner().clone(),
            agent,
            mode,
            delay,
            per_map.unwrap_or_else(|| serde_json::json!({})),
        )
    } else {
        let resolved = crate::catalog::resolve_agent(&agent)
            .ok_or_else(|| format!("Unknown agent '{agent}'"))?;
        Ok(serde_json::json!({
            "ok": true,
            "running": false,
            "status": "dry-run",
            "agent": resolved.name,
            "message": format!(
                "DRY-RUN: would {mode} {} when agent select starts. Enable live actions in Settings to arm it.",
                resolved.name
            )
        }))
    }
}

#[tauri::command]
pub fn instalock_stop(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    state.instalock.stop();
    serde_json::json!({ "ok": true, "running": false })
}

#[tauri::command]
pub fn instalock_status(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    state.instalock.status()
}

#[tauri::command]
pub async fn profile(
    state: State<'_, Arc<AppState>>,
    puuid: String,
) -> Result<serde_json::Value, String> {
    crate::career::profile(state.inner(), &puuid).await
}

#[tauri::command]
pub async fn match_detail(
    state: State<'_, Arc<AppState>>,
    match_id: String,
    subject: Option<String>,
) -> Result<serde_json::Value, String> {
    crate::career::match_detail(state.inner(), &match_id, subject.as_deref()).await
}

#[tauri::command]
pub async fn performance(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let settings = state.settings_snapshot();
    let mut owner = None;
    if let Ok(session) = state.riot.session(&settings.region).await {
        owner = Some(session.puuid.clone());
        crate::history::remember_account(state.inner(), &session.puuid, None);
        let _ = crate::history::refresh(state.inner(), &session).await;
    }
    let mut payload = crate::history::payload(state.inner(), owner.as_deref());
    payload["currentAccountPuuid"] = serde_json::json!(owner);
    payload["isCurrentAccount"] = serde_json::json!(owner.is_some());
    payload["manualSessions"] = crate::sessions::list(state.inner());
    payload["matchMeta"] = crate::match_notes::all(state.inner());
    Ok(payload)
}

#[tauri::command]
pub fn sessions_list(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    crate::sessions::list(state.inner())
}

#[tauri::command]
pub fn session_start(
    state: State<'_, Arc<AppState>>,
    goal_type: Option<String>,
    target: Option<i64>,
) -> Result<serde_json::Value, String> {
    crate::sessions::start(state.inner(), goal_type.as_deref(), target)
}

#[tauri::command]
pub fn session_end(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    crate::sessions::end(state.inner())
}

#[tauri::command]
pub fn session_delete(
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<serde_json::Value, String> {
    crate::sessions::delete(state.inner(), &id)
}

#[tauri::command]
pub fn match_note(
    state: State<'_, Arc<AppState>>,
    match_id: String,
) -> serde_json::Value {
    crate::match_notes::get(state.inner(), &match_id)
}

#[tauri::command]
pub fn match_note_update(
    state: State<'_, Arc<AppState>>,
    match_id: String,
    note: String,
    tags: Vec<String>,
) -> Result<serde_json::Value, String> {
    crate::match_notes::update(state.inner(), &match_id, &note, tags)
}

#[tauri::command]
pub async fn remote_start(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    crate::remote::start(state.inner()).await
}

#[tauri::command]
pub fn remote_info(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    state.remote.info()
}

#[tauri::command]
pub fn encounters(
    state: State<'_, Arc<AppState>>,
    scope: Option<String>,
) -> serde_json::Value {
    crate::encounters::list(state.inner(), None, scope.as_deref() == Some("all"))
}

#[tauri::command]
pub fn encounter(
    state: State<'_, Arc<AppState>>,
    puuid: String,
) -> serde_json::Value {
    crate::encounters::one(state.inner(), None, &puuid)
}

#[tauri::command]
pub async fn inventory(
    state: State<'_, Arc<AppState>>,
    refresh: Option<bool>,
) -> Result<serde_json::Value, String> {
    crate::inventory::snapshot(state.inner(), refresh.unwrap_or(false)).await
}

#[tauri::command]
pub async fn store_snapshot(
    state: State<'_, Arc<AppState>>,
    refresh: Option<bool>,
) -> Result<serde_json::Value, String> {
    crate::store::snapshot(state.inner(), refresh.unwrap_or(false)).await
}

#[tauri::command]
pub async fn loadout_snapshot(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    crate::loadout::snapshot(state.inner()).await
}

#[tauri::command]
pub async fn loadout_set_skin(
    state: State<'_, Arc<AppState>>,
    weapon_id: String,
    skin_id: String,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    crate::loadout::set_skin(state.inner(), &weapon_id, &skin_id, dry_run).await
}

#[tauri::command]
pub async fn contracts_snapshot(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    crate::contracts::snapshot(state.inner()).await
}

#[tauri::command]
pub async fn contract_activate(
    state: State<'_, Arc<AppState>>,
    contract_id: String,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    crate::contracts::activate(state.inner(), &contract_id, dry_run).await
}

#[tauri::command]
pub async fn leaderboard(
    state: State<'_, Arc<AppState>>,
    query: Option<String>,
    size: Option<u32>,
) -> Result<serde_json::Value, String> {
    crate::leaderboard::snapshot(state.inner(), query, size).await
}

#[tauri::command]
pub async fn account_overview(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    crate::account::snapshot(state.inner()).await
}

#[tauri::command]
pub async fn party_details(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    crate::party::details(state.inner()).await
}

#[tauri::command]
pub async fn party_ready(
    state: State<'_, Arc<AppState>>,
    ready: bool,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    crate::party::set_ready(state.inner(), ready, dry_run).await
}

#[tauri::command]
pub async fn party_accessibility(
    state: State<'_, Arc<AppState>>,
    open: bool,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    crate::party::set_accessibility(state.inner(), open, dry_run).await
}

#[tauri::command]
pub async fn party_invite(
    state: State<'_, Arc<AppState>>,
    name: String,
    tag: String,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    crate::party::invite(state.inner(), &name, &tag, dry_run).await
}

#[tauri::command]
pub async fn party_generate_code(
    state: State<'_, Arc<AppState>>,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    crate::party::generate_code(state.inner(), dry_run).await
}

#[tauri::command]
pub async fn party_join_code(
    state: State<'_, Arc<AppState>>,
    code: String,
    dry_run: bool,
) -> Result<serde_json::Value, String> {
    crate::party::join_code(state.inner(), &code, dry_run).await
}

#[tauri::command]
pub async fn offline_status(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    Ok(state.offline.status().await)
}

#[tauri::command]
pub async fn offline_start(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    state.offline.start().await
}

#[tauri::command]
pub async fn offline_stop(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    state.offline.stop().await
}

#[tauri::command]
pub async fn offline_launch(
    state: State<'_, Arc<AppState>>,
    status: Option<String>,
) -> Result<serde_json::Value, String> {
    state.offline.launch(status.as_deref()).await
}

#[tauri::command]
pub async fn offline_set_status(
    state: State<'_, Arc<AppState>>,
    status: String,
) -> Result<serde_json::Value, String> {
    state.offline.set_status(&status).await
}

#[tauri::command]
pub async fn offline_set_enabled(
    state: State<'_, Arc<AppState>>,
    enabled: bool,
) -> Result<serde_json::Value, String> {
    state.offline.set_enabled(enabled).await
}
