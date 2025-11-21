pub mod interface;
pub mod manager;
pub mod resource;

use tauri::State;
use crate::manager::{LightingManager, Device};
use crate::manager::inventory::list_effects;

#[tauri::command]
fn scan_devices(manager: State<LightingManager>) -> Vec<Device> {
    manager.scan_devices()
}

#[tauri::command]
fn get_effects() -> Vec<&'static str> {
    list_effects()
}

#[tauri::command]
fn set_effect(port: String, effect: String, manager: State<LightingManager>) -> Result<(), String> {
    manager.start_effect(&port, &effect)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(LightingManager::new())
        .invoke_handler(tauri::generate_handler![scan_devices, get_effects, set_effect])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
