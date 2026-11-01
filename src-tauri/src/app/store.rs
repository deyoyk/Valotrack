use std::path::{Path, PathBuf};

use serde_json::Value;

pub fn path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.json"))
}

pub fn read(dir: &Path, name: &str) -> Value {
    std::fs::read_to_string(path(dir, name))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| Value::Object(Default::default()))
}

pub fn write(dir: &Path, name: &str, value: &Value) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    let target = path(dir, name);
    let temp = target.with_extension("json.tmp");
    std::fs::write(&temp, raw).map_err(|e| e.to_string())?;
    std::fs::rename(temp, target).map_err(|e| e.to_string())
}
