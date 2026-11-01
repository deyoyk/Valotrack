use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStatus {
    pub available: bool,
    pub lockfile_path: String,
    pub region: Option<String>,
    pub error: Option<String>,
}

pub fn local_appdata() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_default()
}

pub fn lockfile_path() -> PathBuf {
    local_appdata()
        .join("Riot Games")
        .join("Riot Client")
        .join("Config")
        .join("lockfile")
}

pub fn client_status() -> ClientStatus {
    let path = lockfile_path();
    let text = std::fs::read_to_string(&path).ok();
    let pid = text
        .as_deref()
        .and_then(|t| t.split(':').nth(1))
        .and_then(|p| p.parse::<u32>().ok());
    let available = pid.map(process_exists).unwrap_or(false);
    ClientStatus {
        available,
        lockfile_path: path.display().to_string(),
        region: detect_region(),
        error: if available {
            None
        } else {
            Some("Riot Client is not running".into())
        },
    }
}

pub fn process_exists(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return false;
            }
            let mut code = 0u32;
            let ok = GetExitCodeProcess(handle, &mut code) != 0;
            CloseHandle(handle);
            ok && code == STILL_ACTIVE as u32
        }
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
        true
    }
}

pub fn detect_region() -> Option<String> {
    let path = local_appdata()
        .join("VALORANT")
        .join("Saved")
        .join("Logs")
        .join("ShooterGame.log");
    let text = std::fs::read_to_string(path).ok()?;
    let mut pd = None;
    let mut glz = None;
    for line in text.lines().rev().take(400_000) {
        if pd.is_none() && line.contains(".a.pvp.net/account-xp/v1/") {
            pd = line
                .split(".a.pvp.net/account-xp/v1/")
                .next()
                .and_then(|left| left.split('.').last())
                .map(str::to_string);
        }
        if glz.is_none() && line.contains("https://glz-") {
            let rest = line.split("https://glz-").nth(1)?;
            let mut parts = rest.split('.');
            let a = parts.next()?;
            let b = parts.next()?;
            glz = Some(format!("{a}.{b}"));
        }
        if pd.is_some() && glz.is_some() {
            break;
        }
    }
    pd.filter(|p| !p.is_empty())
}

pub async fn party_state() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "available": false,
        "message": "Riot Client is not running"
    }))
}
