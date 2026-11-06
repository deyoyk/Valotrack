#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit = "256"]

mod account;
mod actions;
mod app;
mod career;
mod catalog;
pub mod cli;
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
mod soundboard;
mod store;
mod valo;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

pub use app::AppState;

static QUITTING: AtomicBool = AtomicBool::new(false);

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

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
            SendMessageW(hwnd.0, WM_SETICON, ICON_BIG as usize, big as isize);
        }
        let small = if small.is_null() { big } else { small };
        if !small.is_null() {
            SendMessageW(hwnd.0, WM_SETICON, ICON_SMALL as usize, small as isize);
            SendMessageW(hwnd.0, WM_SETICON, ICON_SMALL2 as usize, small as isize);
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

#[cfg(windows)]
const UPDATER_BYTES: &[u8] = include_bytes!("updater.exe");

#[cfg(windows)]
fn updater_log(stage_dir: &std::path::Path, message: &str) {
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(stage_dir.join("launch.log"))
    {
        let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{stamp}] {message}");
    }
}

#[cfg(windows)]
fn launch_updater(data_dir: &std::path::Path) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    // Stage a copy tagged with the app version so a still-running updater from an
    // earlier launch cannot lock the file that has to be replaced.
    let stage_dir = data_dir.join("updater");
    if std::fs::create_dir_all(&stage_dir).is_err() {
        return;
    }
    // Fingerprint the payload so a rebuilt updater.exe is staged again even when the
    // app version did not change.
    let mut fingerprint: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in UPDATER_BYTES {
        fingerprint ^= *byte as u64;
        fingerprint = fingerprint.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let file_name = format!(
        "updater-{}-{fingerprint:016x}.exe",
        env!("CARGO_PKG_VERSION")
    );
    let exe_path = stage_dir.join(&file_name);
    let staged_len = std::fs::metadata(&exe_path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    if staged_len != UPDATER_BYTES.len() as u64 {
        match std::fs::write(&exe_path, UPDATER_BYTES) {
            Ok(()) => updater_log(&stage_dir, &format!("staged {file_name}")),
            Err(err) => {
                updater_log(&stage_dir, &format!("stage failed: {err}"));
                return;
            }
        }
    }

    // Drop copies staged by older builds. Files still in use stay until a later run.
    if let Ok(entries) = std::fs::read_dir(&stage_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("updater-")
                && name.ends_with(".exe")
                && name.as_ref() != file_name.as_str()
            {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    // The updater is a console program. CREATE_NO_WINDOW stops Windows from
    // allocating a console window. Anything it prints goes to a log file instead of
    // a console.
    let output_log = std::fs::File::create(stage_dir.join("updater-output.log")).ok();
    let mut command = std::process::Command::new(&exe_path);
    command.stdin(std::process::Stdio::null());
    match output_log {
        Some(file) => {
            let stderr = file.try_clone().ok();
            command.stdout(std::process::Stdio::from(file));
            command.stderr(match stderr {
                Some(stderr) => std::process::Stdio::from(stderr),
                None => std::process::Stdio::null(),
            });
        }
        None => {
            command
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }
    }
    command.creation_flags(CREATE_NO_WINDOW);
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    {
        command.current_dir(dir);
    }
    match command.spawn() {
        Ok(mut child) => {
            updater_log(
                &stage_dir,
                &format!("started {} pid={}", exe_path.display(), child.id()),
            );
            let log_dir = stage_dir.clone();
            std::thread::spawn(move || {
                let message = match child.wait() {
                    Ok(status) => format!("exited {status}"),
                    Err(err) => format!("wait failed: {err}"),
                };
                updater_log(&log_dir, &message);
            });
        }
        Err(err) => updater_log(
            &stage_dir,
            &format!("spawn failed: {err} (raw os error {:?})", err.raw_os_error()),
        ),
    }
}

#[cfg(not(windows))]
fn launch_updater(_data_dir: &std::path::Path) {}

fn spawn_updater_background(data_dir: std::path::PathBuf) {
    let _ = std::thread::Builder::new()
        .name("updater-launch".into())
        .spawn(move || launch_updater(&data_dir));
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
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let handle = app.handle();
            let state = Arc::new(AppState::new(handle)?);
            refresh_shell_icons(&state.data_dir);
            spawn_updater_background(state.data_dir.clone());
            app.manage(state.clone());
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(icon) =
                    tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
                {
                    let _ = window.set_icon(icon);
                }
                #[cfg(windows)]
                apply_taskbar_icon(&window);
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        if !QUITTING.load(Ordering::SeqCst) {
                            api.prevent_close();
                            hide_main_window(&handle);
                        }
                    }
                });
            }
            let show_item = MenuItem::with_id(app, "show", "Open Valotrack", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide to tray", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Exit Valotrack", true, None::<&str>)?;
            let tray_menu =
                Menu::with_items(app, &[&show_item, &hide_item, &separator, &quit_item])?;
            let mut tray = TrayIconBuilder::with_id("valotrack-tray")
                .tooltip("Valotrack - local VALORANT companion")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main_window(app),
                    "hide" => hide_main_window(app),
                    "quit" => {
                        QUITTING.store(true, Ordering::SeqCst);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                });
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
                .ok()
                .or_else(|| app.default_window_icon().cloned());
            if let Some(icon) = tray_icon {
                tray = tray.icon(icon);
            }
            let tray = tray.build(app)?;
            app.manage(tray);
            discord::spawn(state.clone());
            crate::live::prewarm(&state);
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
            commands::soundboard_snapshot,
            commands::soundboard_import,
            commands::soundboard_remove,
            commands::soundboard_play,
            commands::soundboard_stop,
            commands::soundboard_stop_all,
            commands::soundboard_test,
            commands::soundboard_set_device,
            commands::soundboard_set_volume,
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
