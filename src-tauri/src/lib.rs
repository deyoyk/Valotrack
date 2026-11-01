#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit = "256"]

mod actions;
mod app;
mod career;
mod catalog;
mod commands;
mod encounters;
mod history;
mod instalock;
mod inventory;
mod live;
mod riot;
mod valo;

use std::sync::Arc;

use tauri::Manager;

pub use app::AppState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            let state = AppState::new(handle)?;
            app.manage(Arc::new(state));
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Valotrack");
}
