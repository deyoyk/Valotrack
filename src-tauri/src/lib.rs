#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit = "256"]

mod actions;
mod app;
mod career;
mod catalog;
mod commands;
mod discord;
mod encounters;
mod history;
mod instalock;
mod inventory;
mod live;
mod match_notes;
mod offline;
mod offline_client;
mod remote;
mod riot;
mod sessions;
mod valo;

use std::sync::Arc;

use tauri::Manager;

pub use app::AppState;

pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == offline::HELPER_FLAG) {
        let data_dir = args
            .iter()
            .position(|arg| arg == offline::DATA_DIR_FLAG)
            .and_then(|index| args.get(index + 1))
            .cloned()
            .unwrap_or_default();
        let _ = offline::run_helper(&data_dir);
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let handle = app.handle();
            let state = Arc::new(AppState::new(handle)?);
            app.manage(state.clone());
            discord::spawn(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health,
            commands::client_status,
            commands::live_board,
            commands::queue_state,
            commands::agents,
            commands::settings_get,
            commands::settings_set,
            commands::queue_action,
            commands::dodge,
            commands::check_side,
            commands::instalock_now,
            commands::instalock_start,
            commands::instalock_stop,
            commands::instalock_status,
            commands::profile,
            commands::match_detail,
            commands::performance,
            commands::encounters,
            commands::encounter,
            commands::inventory,
            commands::offline_status,
            commands::offline_start,
            commands::offline_stop,
            commands::offline_launch,
            commands::offline_set_status,
            commands::offline_set_enabled,
            commands::sessions_list,
            commands::session_start,
            commands::session_end,
            commands::session_delete,
            commands::match_note,
            commands::match_note_update,
            commands::remote_start,
            commands::remote_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Valotrack");
}
