pub mod interface;
pub mod manager;
pub mod resource;

use crate::interface::effect::{EffectParam, EffectParamKind};
use crate::manager::inventory::list_effects;
use crate::manager::{Device, LightingManager};
#[cfg(target_os = "windows")]
use crate::resource::screen::windows::{
    get_capture_fps as get_windows_capture_fps,
    get_capture_scale_percent,
    list_displays as list_windows_displays,
    set_capture_fps as set_windows_capture_fps,
    set_capture_scale_percent,
    DisplayInfo as WindowsDisplayInfo,
};
use tauri::State;

#[tauri::command]
async fn scan_devices(manager: State<'_, LightingManager>) -> Result<Vec<Device>, String> {
    Ok(manager.scan_devices())
}

use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "type")]
enum EffectParamInfo {
    #[serde(rename = "slider")]
    Slider {
        key: &'static str,
        label: &'static str,
        min: f64,
        max: f64,
        step: f64,
        default: f64,
    },
    #[serde(rename = "select")]
    Select {
        key: &'static str,
        label: &'static str,
        default: f64,
        options: Vec<SelectOptionInfo>,
    },
}

#[derive(Serialize)]
struct SelectOptionInfo {
    label: String,
    value: f64,
}

impl From<&'static EffectParam> for EffectParamInfo {
    fn from(param: &'static EffectParam) -> Self {
        match &param.kind {
            EffectParamKind::Slider {
                min,
                max,
                step,
                default,
            } => EffectParamInfo::Slider {
                key: param.key,
                label: param.label,
                min: *min,
                max: *max,
                step: *step,
                default: *default,
            },
            EffectParamKind::Select { default, options } => {
                let resolved = match options.resolve() {
                    Ok(list) => list,
                    Err(err) => {
                        eprintln!(
                            "[effects] Failed to resolve select options for '{}': {}",
                            param.key, err
                        );
                        Vec::new()
                    }
                };

                let mut default_value = *default;
                if !resolved.is_empty()
                    && !resolved
                        .iter()
                        .any(|option| (option.value - default_value).abs() < f64::EPSILON)
                {
                    default_value = resolved[0].value;
                }

                let options = resolved
                    .into_iter()
                    .map(|option| SelectOptionInfo {
                        label: option.label,
                        value: option.value,
                    })
                    .collect();

                EffectParamInfo::Select {
                    key: param.key,
                    label: param.label,
                    default: default_value,
                    options,
                }
            }
        }
    }
}

#[derive(Serialize)]
struct EffectInfo {
    id: &'static str,
    name: &'static str,
    description: Option<&'static str>,
    group: Option<&'static str>,
    params: Vec<EffectParamInfo>,
}

#[cfg(target_os = "windows")]
type DisplayInfoResponse = WindowsDisplayInfo;

#[cfg(not(target_os = "windows"))]
#[derive(Serialize)]
struct DisplayInfoResponse {
    index: usize,
    name: String,
    width: u32,
    height: u32,
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
            params: e.params.iter().map(EffectParamInfo::from).collect(),
        })
        .collect()
}

#[tauri::command]
fn get_displays() -> Vec<DisplayInfoResponse> {
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
fn set_effect(
    port: String,
    effect_id: String,
    manager: State<LightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    manager.start_effect(&port, &effect_id, app_handle)
}

#[tauri::command]
fn update_effect_params(
    port: String,
    params: serde_json::Value,
    manager: State<LightingManager>,
) -> Result<(), String> {
    manager.update_effect_params(&port, params)
}

#[tauri::command]
fn set_brightness(
    port: String,
    brightness: u8,
    manager: State<LightingManager>,
) -> Result<(), String> {
    manager.set_brightness(&port, brightness)
}

#[tauri::command]
fn set_capture_scale(percent: u8) {
    #[cfg(target_os = "windows")]
    set_capture_scale_percent(percent);
}

#[tauri::command]
fn get_capture_scale() -> u8 {
    #[cfg(target_os = "windows")]
    return get_capture_scale_percent();
    #[cfg(not(target_os = "windows"))]
    100
}

#[tauri::command]
fn set_capture_fps(fps: u8) {
    #[cfg(target_os = "windows")]
    set_windows_capture_fps(fps);
}

#[tauri::command]
fn get_capture_fps() -> u8 {
    #[cfg(target_os = "windows")]
    return get_windows_capture_fps();
    #[cfg(not(target_os = "windows"))]
    30
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(LightingManager::new())
        .invoke_handler(tauri::generate_handler![
            scan_devices,
            get_effects,
            get_displays,
            set_effect,
            update_effect_params,
            set_brightness,
            set_capture_scale,
            get_capture_scale,
            set_capture_fps,
            get_capture_fps
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
