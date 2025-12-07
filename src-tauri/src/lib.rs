pub mod interface;
pub mod manager;
pub mod resource;
pub mod api;

use crate::manager::LightingManager;
use crate::api::commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(LightingManager::new())
        .invoke_handler(tauri::generate_handler![
            commands::scan_devices,
            commands::get_effects,
            commands::get_displays,
            commands::set_effect,
            commands::update_effect_params,
            commands::set_brightness,
            commands::set_capture_scale,
            commands::get_capture_scale,
            commands::set_capture_fps,
            commands::get_capture_fps,
            commands::set_capture_method,
            commands::get_capture_method
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
