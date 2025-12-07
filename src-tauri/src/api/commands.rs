use tauri::State;
use crate::manager::{Device, LightingManager};
use crate::manager::inventory::list_effects;
use crate::api::dto::{EffectInfo, EffectParamInfo};

#[cfg(target_os = "windows")]
use crate::resource::screen::windows::{
    get_capture_fps as get_windows_capture_fps,
    get_capture_method as get_windows_capture_method,
    get_capture_scale_percent,
    list_displays as list_windows_displays,
    set_capture_fps as set_windows_capture_fps,
    set_capture_method as set_windows_capture_method,
    set_capture_scale_percent,
    CaptureMethod,
    DisplayInfo as WindowsDisplayInfo,
};

#[cfg(target_os = "windows")]
pub type DisplayInfoResponse = WindowsDisplayInfo;

#[cfg(not(target_os = "windows"))]
use crate::api::dto::DisplayInfoResponse;

#[tauri::command]
pub async fn scan_devices(manager: State<'_, LightingManager>) -> Result<Vec<Device>, String> {
    Ok(manager.scan_devices())
}

#[tauri::command]
pub fn get_effects() -> Vec<EffectInfo> {
    list_effects()
        .into_iter()
        .map(|e| EffectInfo {
            id: e.id,
            name: e.name,
            description: e.description,
            group: e.group,
            icon: e.icon,
            params: e.params.iter().map(EffectParamInfo::from).collect(),
        })
        .collect()
}

#[tauri::command]
pub fn get_displays() -> Vec<DisplayInfoResponse> {
    #[cfg(target_os = "windows")]
    {
        match list_windows_displays() {
            Ok(displays) => displays,
            Err(err) => {
                eprintln!("[screen] Failed to enumerate displays: {}", err);
                Vec::new()
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

#[tauri::command]
pub fn set_effect(
    port: String,
    effect_id: String,
    manager: State<LightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    manager.start_effect(&port, &effect_id, app_handle)
}

#[tauri::command]
pub fn update_effect_params(
    port: String,
    params: serde_json::Value,
    manager: State<LightingManager>,
) -> Result<(), String> {
    manager.update_effect_params(&port, params)
}

#[tauri::command]
pub fn set_brightness(
    port: String,
    brightness: u8,
    manager: State<LightingManager>,
) -> Result<(), String> {
    manager.set_brightness(&port, brightness)
}

#[tauri::command]
pub fn set_capture_scale(percent: u8) {
    #[cfg(target_os = "windows")]
    set_capture_scale_percent(percent);
}

#[tauri::command]
pub fn get_capture_scale() -> u8 {
    #[cfg(target_os = "windows")]
    return get_capture_scale_percent();
    #[cfg(not(target_os = "windows"))]
    100
}

#[tauri::command]
pub fn set_capture_fps(fps: u8) {
    #[cfg(target_os = "windows")]
    set_windows_capture_fps(fps);
}

#[tauri::command]
pub fn get_capture_fps() -> u8 {
    #[cfg(target_os = "windows")]
    return get_windows_capture_fps();
    #[cfg(not(target_os = "windows"))]
    30
}

#[tauri::command]
pub fn set_capture_method(method: String) {
    #[cfg(target_os = "windows")]
    {
        if let Ok(m) = method.parse::<CaptureMethod>() {
            set_windows_capture_method(m);
        }
    }
}

#[tauri::command]
pub fn get_capture_method() -> String {
    #[cfg(target_os = "windows")]
    return get_windows_capture_method().to_string();
    #[cfg(not(target_os = "windows"))]
    "dxgi".to_string()
}
