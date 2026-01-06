use std::io::Write;
use tauri::Manager;

use crate::api::dto::AppConfigDto;
use crate::manager::PersistedDeviceConfig;

fn app_config_file_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let base = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve app config dir: {e}"))?;
    std::fs::create_dir_all(&base)
        .map_err(|e| format!("Failed to create app config dir '{base:?}': {e}"))?;
    Ok(base.join("app.json"))
}

fn devices_dir_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let base = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve app config dir: {e}"))?;
    let dir = base.join("devices");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create devices dir '{dir:?}': {e}"))?;
    Ok(dir)
}

fn device_file_path(app_handle: &tauri::AppHandle, device_id: &str) -> Result<std::path::PathBuf, String> {
    let dir = devices_dir_path(app_handle)?;

    // Keep filenames filesystem-friendly.
    let safe = device_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();

    Ok(dir.join(format!("{safe}.json")))
}

pub fn load_app_config(app_handle: &tauri::AppHandle) -> Result<AppConfigDto, String> {
    let path = app_config_file_path(app_handle)?;

    if !path.exists() {
        return Ok(AppConfigDto::default_for_platform());
    }

    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read app config '{path:?}': {e}"))?;

    serde_json::from_str::<AppConfigDto>(&raw)
        .map_err(|e| format!("Failed to parse app config '{path:?}': {e}"))
}

pub fn save_app_config(app_handle: &tauri::AppHandle, config: &AppConfigDto) -> Result<(), String> {
    let path = app_config_file_path(app_handle)?;

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize app config: {e}"))?;

    // Atomic-ish write: write to temp then rename.
    let tmp = path.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&tmp)
            .map_err(|e| format!("Failed to create app config '{tmp:?}': {e}"))?;
        f.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write app config '{tmp:?}': {e}"))?;
        f.flush()
            .map_err(|e| format!("Failed to flush app config '{tmp:?}': {e}"))?;
    }
    std::fs::rename(&tmp, &path)
        .map_err(|e| format!("Failed to move app config '{tmp:?}' -> '{path:?}': {e}"))?;

    Ok(())
}

pub fn load_device_config(
    app_handle: &tauri::AppHandle,
    device_id: &str,
) -> Result<Option<PersistedDeviceConfig>, String> {
    let path = device_file_path(app_handle, device_id)?;

    if !path.exists() {
        return Ok(None);
    }

    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read device config '{path:?}': {e}"))?;

    let parsed = serde_json::from_str::<PersistedDeviceConfig>(&raw)
        .map_err(|e| format!("Failed to parse device config '{path:?}': {e}"))?;

    Ok(Some(parsed))
}

pub fn save_device_config(
    app_handle: &tauri::AppHandle,
    device_id: &str,
    config: &PersistedDeviceConfig,
) -> Result<(), String> {
    let path = device_file_path(app_handle, device_id)?;

    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize device config: {e}"))?;

    let tmp = path.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&tmp)
            .map_err(|e| format!("Failed to create device config '{tmp:?}': {e}"))?;
        f.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write device config '{tmp:?}': {e}"))?;
        f.flush()
            .map_err(|e| format!("Failed to flush device config '{tmp:?}': {e}"))?;
    }
    std::fs::rename(&tmp, &path)
        .map_err(|e| format!("Failed to move device config '{tmp:?}' -> '{path:?}': {e}"))?;

    Ok(())
}
