pub mod interface;
pub mod manager;
pub mod resource;

use tauri::State;
use crate::manager::{LightingManager, Device};

#[tauri::command]
fn scan_devices(manager: State<LightingManager>) -> Vec<Device> {
    manager.scan_devices()
}

#[tauri::command]
fn set_rainbow(port: String, manager: State<LightingManager>) -> Result<(), String> {
    manager.start_effect(&port, "Rainbow")
}

#[tauri::command]
fn turn_off(port: String, manager: State<LightingManager>) -> Result<(), String> {
    manager.turn_off(&port)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(LightingManager::new())
        .invoke_handler(tauri::generate_handler![scan_devices, set_rainbow, turn_off])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
