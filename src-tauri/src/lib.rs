#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit = "256"]

mod actions;
mod account;
mod app;
mod career;
mod catalog;
mod commands;
mod content;
mod contracts;
mod discord;
mod encounters;
mod history;
mod instalock;
mod inventory;
mod leaderboard;
mod live;
mod loadout;
mod match_notes;
mod metrics;
mod offline;
mod offline_client;
mod party;
mod remote;
mod riot;
mod sessions;
mod social;
mod store;
mod valo;

use std::sync::Arc;

use tauri::Manager;

pub use app::AppState;

#[cfg(windows)]
fn apply_taskbar_icon(window: &tauri::WebviewWindow) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ExtractIconExW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageW, ICON_BIG, ICON_SMALL, ICON_SMALL2, WM_SETICON,
    };

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let wide = exe
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();

    unsafe {
        let mut big = std::ptr::null_mut();
        let mut small = std::ptr::null_mut();
        if ExtractIconExW(wide.as_ptr(), 0, &mut big, &mut small, 1) == 0 {
            return;
        }
        if !big.is_null() {
            SendMessageW(
                hwnd.0,
                WM_SETICON,
                ICON_BIG as usize,
                big as isize,
            );
        }
        let small = if small.is_null() { big } else { small };
        if !small.is_null() {
            SendMessageW(
                hwnd.0,
                WM_SETICON,
                ICON_SMALL as usize,
                small as isize,
            );
            SendMessageW(
                hwnd.0,
                WM_SETICON,
                ICON_SMALL2 as usize,
                small as isize,
            );
        }
    }
}

fn refresh_shell_icons(data_dir: &std::path::Path) {
    #[cfg(windows)]
    {
        let marker = data_dir.join("shell-icon-version.txt");
        let version = env!("CARGO_PKG_VERSION");
        if std::fs::read_to_string(&marker).ok().as_deref() == Some(version) {
            return;
        }
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("ie4uinit.exe")
            .arg("-show")
            .creation_flags(0x08000000)
            .status();
        unsafe {
            windows_sys::Win32::UI::Shell::SHChangeNotify(
                windows_sys::Win32::UI::Shell::SHCNE_ASSOCCHANGED as i32,
                windows_sys::Win32::UI::Shell::SHCNF_IDLIST,
                std::ptr::null(),
                std::ptr::null(),
            );
        }
        let _ = std::fs::write(marker, version);
    }
    #[cfg(not(windows))]
    {
        let _ = data_dir;
    }
}

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
            refresh_shell_icons(&state.data_dir);
            app.manage(state.clone());
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(icon) =
                    tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
                {
                    let _ = window.set_icon(icon);
                }
                #[cfg(windows)]
                apply_taskbar_icon(&window);
            }
            discord::spawn(state.clone());
            metrics::spawn(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health,
            commands::client_status,
            commands::concurrent_users,
            commands::live_board,
            commands::queue_state,
            commands::agents,
            commands::owned_agents,
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
            commands::store_snapshot,
            commands::loadout_snapshot,
            commands::loadout_set_skin,
            commands::contracts_snapshot,
            commands::contract_activate,
            commands::leaderboard,
            commands::account_overview,
            commands::party_details,
            commands::party_ready,
            commands::party_accessibility,
            commands::party_invite,
            commands::party_generate_code,
            commands::party_join_code,
            commands::social_snapshot,
            commands::social_messages,
            commands::social_send,
            commands::social_send_request,
            commands::social_remove_request,
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
