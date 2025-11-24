pub mod interface;
pub mod manager;
pub mod resource;

use tauri::State;
use crate::manager::{LightingManager, Device};
use crate::manager::inventory::list_effects;

#[tauri::command]
async fn scan_devices(manager: State<'_, LightingManager>) -> Result<Vec<Device>, String> {
    Ok(manager.scan_devices())
}

use serde::Serialize;

#[derive(Serialize)]
struct EffectInfo {
    id: &'static str,
    name: &'static str,
    description: Option<&'static str>,
    group: Option<&'static str>,
}

#[tauri::command]
fn get_effects() -> Vec<EffectInfo> {
    list_effects()
        .into_iter()
        .map(|e| EffectInfo {
            id: e.id,
            name: e.name,
            description: e.description,
            group: e.group,
        })
        .collect()
}

#[tauri::command]
fn set_effect(port: String, effect_id: String, manager: State<LightingManager>, app_handle: tauri::AppHandle) -> Result<(), String> {
    manager.start_effect(&port, &effect_id, app_handle)
}

#[tauri::command]
fn update_effect_params(port: String, params: serde_json::Value, manager: State<LightingManager>) -> Result<(), String> {
    manager.update_effect_params(&port, params)
}

#[tauri::command]
fn set_brightness(port: String, brightness: u8, manager: State<LightingManager>) -> Result<(), String> {
    manager.set_brightness(&port, brightness)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .manage(LightingManager::new())
    .invoke_handler(tauri::generate_handler![scan_devices, get_effects, set_effect, update_effect_params, set_brightness])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
