use tauri::State;
use crate::manager::{Device, LightingManager};
use crate::manager::inventory::list_effects;
use crate::api::dto::{EffectInfo, EffectParamInfo, SystemInfoResponse};

#[cfg(any(target_os = "windows", target_os = "macos"))]
use once_cell::sync::Lazy;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::sync::Mutex;

#[cfg(any(target_os = "windows", target_os = "macos"))]
use tauri::Manager;

use crate::resource::screen::{
    get_capture_fps as get_screen_capture_fps,
    get_capture_method as get_screen_capture_method,
    get_capture_scale_percent,
    list_displays as list_screen_displays,
    set_capture_fps as set_screen_capture_fps,
    set_capture_method as set_screen_capture_method,
    set_capture_scale_percent,
    CaptureMethod,
    DisplayInfo,
};

#[cfg(target_os = "windows")]
use window_vibrancy::{
    apply_acrylic, apply_blur, apply_mica, apply_tabbed, clear_acrylic, clear_blur, clear_mica,
    clear_tabbed,
};

#[cfg(target_os = "macos")]
use window_vibrancy::{
    apply_vibrancy, clear_vibrancy, NSVisualEffectMaterial, NSVisualEffectState,
};

#[cfg(target_os = "windows")]
use winreg::enums::HKEY_LOCAL_MACHINE;
#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "macos")]
use std::process::Command;
#[cfg(all(unix, not(target_os = "macos"), not(target_os = "windows")))]
use std::fs;

pub type DisplayInfoResponse = DisplayInfo;

#[cfg(any(target_os = "windows", target_os = "macos"))]
static CURRENT_WINDOW_EFFECT: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

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
    match list_screen_displays() {
        Ok(displays) => displays,
        Err(err) => {
            eprintln!("[screen] Failed to enumerate displays: {}", err);
            Vec::new()
        }
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
    set_capture_scale_percent(percent);
}

#[tauri::command]
pub fn get_capture_scale() -> u8 {
    get_capture_scale_percent()
}

#[tauri::command]
pub fn set_capture_fps(fps: u8) {
    set_screen_capture_fps(fps);
}

#[tauri::command]
pub fn get_capture_fps() -> u8 {
    get_screen_capture_fps()
}

#[tauri::command]
pub fn set_capture_method(method: String) {
    if let Ok(m) = method.parse::<CaptureMethod>() {
        set_screen_capture_method(m);
    }
}

#[tauri::command]
pub fn get_capture_method() -> String {
    get_screen_capture_method().to_string()
}

// ============================================================================
// Window background effects - shared API
// ============================================================================

#[tauri::command]
pub fn get_window_effects() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        get_window_effects_windows()
    }
    #[cfg(target_os = "macos")]
    {
        get_window_effects_macos()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Vec::new()
    }
}

#[tauri::command]
pub fn get_window_effect() -> String {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let mut guard = CURRENT_WINDOW_EFFECT.lock().unwrap();
        if guard.is_empty() {
            let default = default_effect_for_platform();
            *guard = default.to_string();
        }
        guard.clone()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        "none".to_string()
    }
}

// ============================================================================
// System info
// ============================================================================

#[cfg(target_os = "windows")]
#[tauri::command]
pub fn get_system_info() -> SystemInfoResponse {
    system_info_windows()
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn get_system_info() -> SystemInfoResponse {
    system_info_macos()
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "windows")))]
#[tauri::command]
pub fn get_system_info() -> SystemInfoResponse {
    system_info_unix()
}

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    all(unix, not(target_os = "macos"), not(target_os = "windows"))
)))]
#[tauri::command]
pub fn get_system_info() -> SystemInfoResponse {
    SystemInfoResponse {
        os_platform: std::env::consts::OS.to_string(),
        os_version: "unknown".to_string(),
        os_build: "unknown".to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

#[tauri::command]
pub fn set_window_effect(effect: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        apply_window_effect_impl(&effect, &app_handle)?;
        let mut guard = CURRENT_WINDOW_EFFECT.lock().unwrap();
        *guard = effect;
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = app_handle;
        Ok(())
    }
}

// Used from lib.rs during app setup
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn initialize_window_effect(app: &tauri::App) {
    let default = default_effect_for_platform();
    let handle = app.handle();

    if let Err(err) = apply_window_effect_impl(default, &handle) {
        eprintln!("[window_effect] Failed to apply default window effect '{}': {}", default, err);
    }

    let mut guard = CURRENT_WINDOW_EFFECT.lock().unwrap();
    *guard = default.to_string();
}

// ============================================================================
// Platform-specific implementation details
// ============================================================================

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
struct WindowsVersion {
    major: u32,
    build: u32,
}

#[cfg(target_os = "windows")]
fn get_windows_version() -> Option<WindowsVersion> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let current_version = hklm
        .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .ok()?;

    let build_str: String = current_version
        .get_value("CurrentBuildNumber")
        .ok()?;
    let build = build_str.parse::<u32>().ok()?;

    // Prefer explicit major version if present, otherwise infer from build.
    let major: u32 = current_version
        .get_value("CurrentMajorVersionNumber")
        .ok()
        .unwrap_or_else(|| if build >= 22000 { 11 } else { 10 });

    Some(WindowsVersion { major, build })
}

#[cfg(target_os = "windows")]
fn get_windows_display_version() -> Option<String> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let current_version = hklm
        .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .ok()?;
    current_version.get_value("DisplayVersion").ok()
}

#[cfg(target_os = "windows")]
fn is_windows_11(ver: &WindowsVersion) -> bool {
    ver.major >= 10 && ver.build >= 22000
}

#[cfg(target_os = "windows")]
fn system_info_windows() -> SystemInfoResponse {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let current_version = hklm
        .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .ok();

    let mut product_name: String = current_version
        .as_ref()
        .and_then(|key| key.get_value("ProductName").ok())
        .unwrap_or_else(|| "Windows".to_string());

    let display_version: String = current_version
        .as_ref()
        .and_then(|key| key.get_value("DisplayVersion").ok())
        .unwrap_or_else(|| "unknown".to_string());

    let build_number: Option<String> = current_version
        .as_ref()
        .and_then(|key| key.get_value("CurrentBuildNumber").ok());
    let ubr: Option<u32> = current_version
        .as_ref()
        .and_then(|key| key.get_value("UBR").ok());

    let build_lab_ex: Option<String> = current_version
        .as_ref()
        .and_then(|key| key.get_value("BuildLabEx").ok());

    let os_build = match (build_number, ubr) {
        (Some(build), Some(ubr)) => format!("{}.{}", build, ubr),
        (Some(build), None) => build,
        (None, _) => {
            if let Some(lab) = build_lab_ex {
                lab.split('.').take(2).collect::<Vec<_>>().join(".")
            } else {
                "unknown".to_string()
            }
        }
    };

    // Heuristic: some Win11 IoT builds still report "Windows 10 ..." in ProductName.
    // If build indicates Win11, normalize the prefix to "Windows 11".
    if let Some(ver) = get_windows_version() {
        if is_windows_11(&ver) && product_name.starts_with("Windows 10 ") {
            if let Some(rest) = product_name.strip_prefix("Windows 10 ") {
                product_name = format!("Windows 11 {}", rest);
            }
        }
    }

    SystemInfoResponse {
        os_platform: product_name,
        os_version: display_version,
        os_build,
        arch: std::env::consts::ARCH.to_string(),
    }
}

#[cfg(target_os = "macos")]
fn system_info_macos() -> SystemInfoResponse {
    let product_name = Command::new("sw_vers")
        .arg("-productName")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "macOS".to_string());

    let product_version = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    let build_version = Command::new("sw_vers")
        .arg("-buildVersion")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    SystemInfoResponse {
        os_platform: product_name,
        os_version: product_version,
        os_build: build_version,
        arch: std::env::consts::ARCH.to_string(),
    }
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "windows")))]
fn system_info_unix() -> SystemInfoResponse {
    let mut os_platform = String::from("Linux");
    let mut os_version = String::from("unknown");
    let mut os_build = String::from("unknown");

    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("PRETTY_NAME=") {
                os_platform = rest.trim_matches('"').to_string();
            } else if let Some(rest) = line.strip_prefix("VERSION=") {
                os_version = rest.trim_matches('"').to_string();
            } else if let Some(rest) = line.strip_prefix("VERSION_ID=") {
                os_build = rest.trim_matches('"').to_string();
            }
        }
    }

    SystemInfoResponse {
        os_platform,
        os_version,
        os_build,
        arch: std::env::consts::ARCH.to_string(),
    }
}

#[cfg(target_os = "windows")]
fn get_window_effects_windows() -> Vec<String> {
    let mut effects = Vec::new();

    let display_version = get_windows_display_version().unwrap_or_else(|| "unknown".to_string());

    if let Some(ver) = get_windows_version() {
        let is_win11 = is_windows_11(&ver);

        // Blur: Windows 7/10; for Windows 11 only when DisplayVersion == 22H1 and build != 22621.*
        let allow_blur = if is_win11 {
            display_version == "22H1" && ver.build != 22621
        } else {
            true
        };
        if allow_blur {
            effects.push("blur".to_string());
        }

        // Acrylic: Windows 10/11, but hide on:
        // - Windows 10 build >= 18362 (v1903+)
        // - Windows 11 build == 22000.*
        let allow_acrylic = if is_win11 {
            ver.build != 22000
        } else if ver.major == 10 {
            ver.build < 18362
        } else {
            false
        };
        if allow_acrylic {
            effects.push("acrylic".to_string());
        }

        // Mica / Tabbed: Windows 11 only.
        if is_win11 {
            effects.push("mica".to_string());
            effects.push("tabbed".to_string());
        }
    } else {
        // Fallback: when version detection fails, keep safest option minimal.
        effects.push("blur".to_string());
    }

    effects
}

#[cfg(target_os = "macos")]
fn get_window_effects_macos() -> Vec<String> {
    [
        "appearanceBased",
        "light",
        "dark",
        "mediumLight",
        "ultraDark",
        "titlebar",
        "selection",
        "menu",
        "popover",
        "sidebar",
        "headerView",
        "sheet",
        "windowBackground",
        "hudWindow",
        "fullScreenUI",
        "tooltip",
        "contentBackground",
        "underWindowBackground",
        "underPageBackground",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[cfg(target_os = "windows")]
fn default_effect_for_platform() -> &'static str {
    if let Some(ver) = get_windows_version() {
        if is_windows_11(&ver) {
            "mica"
        } else {
            "blur"
        }
    } else {
        "blur"
    }
}

#[cfg(target_os = "macos")]
fn default_effect_for_platform() -> &'static str {
    "sidebar"
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn default_effect_for_platform() -> &'static str {
    "none"
}

#[cfg(target_os = "windows")]
fn apply_window_effect_impl(
    effect: &str,
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    // Clear existing effects; ignore errors but log them for debugging.
    if let Err(err) = clear_mica(&window) {
        eprintln!("[window_effect] clear_mica failed: {}", err);
    }
    if let Err(err) = clear_tabbed(&window) {
        eprintln!("[window_effect] clear_tabbed failed: {}", err);
    }
    if let Err(err) = clear_blur(&window) {
        eprintln!("[window_effect] clear_blur failed: {}", err);
    }
    if let Err(err) = clear_acrylic(&window) {
        eprintln!("[window_effect] clear_acrylic failed: {}", err);
    }

    match effect {
        "mica" => apply_mica(&window, None)
            .map_err(|e| format!("Failed to apply mica: {}", e)),
        "micaDark" => apply_mica(&window, Some(true))
            .map_err(|e| format!("Failed to apply micaDark: {}", e)),
        "micaLight" => apply_mica(&window, Some(false))
            .map_err(|e| format!("Failed to apply micaLight: {}", e)),
        "tabbed" => apply_tabbed(&window, None)
            .map_err(|e| format!("Failed to apply tabbed: {}", e)),
        "tabbedDark" => apply_tabbed(&window, Some(true))
            .map_err(|e| format!("Failed to apply tabbedDark: {}", e)),
        "tabbedLight" => apply_tabbed(&window, Some(false))
            .map_err(|e| format!("Failed to apply tabbedLight: {}", e)),
        "blur" => apply_blur(&window, Some((18, 18, 18, 125)))
            .map_err(|e| format!("Failed to apply blur: {}", e)),
        "acrylic" => apply_acrylic(&window, Some((30, 30, 30, 100)))
            .map_err(|e| format!("Failed to apply acrylic: {}", e)),
        other => Err(format!("Unsupported window effect: {}", other)),
    }
}

#[cfg(target_os = "macos")]
fn apply_window_effect_impl(
    effect: &str,
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    if let Err(err) = clear_vibrancy(&window) {
        eprintln!("[window_effect] clear_vibrancy failed: {}", err);
    }

    let material = match effect {
        "appearanceBased" => NSVisualEffectMaterial::AppearanceBased,
        "light" => NSVisualEffectMaterial::Light,
        "dark" => NSVisualEffectMaterial::Dark,
        "mediumLight" => NSVisualEffectMaterial::MediumLight,
        "ultraDark" => NSVisualEffectMaterial::UltraDark,
        "titlebar" => NSVisualEffectMaterial::Titlebar,
        "selection" => NSVisualEffectMaterial::Selection,
        "menu" => NSVisualEffectMaterial::Menu,
        "popover" => NSVisualEffectMaterial::Popover,
        "sidebar" => NSVisualEffectMaterial::Sidebar,
        "headerView" => NSVisualEffectMaterial::HeaderView,
        "sheet" => NSVisualEffectMaterial::Sheet,
        "windowBackground" => NSVisualEffectMaterial::WindowBackground,
        "hudWindow" => NSVisualEffectMaterial::HudWindow,
        "fullScreenUI" => NSVisualEffectMaterial::FullScreenUI,
        "tooltip" => NSVisualEffectMaterial::Tooltip,
        "contentBackground" => NSVisualEffectMaterial::ContentBackground,
        "underWindowBackground" => NSVisualEffectMaterial::UnderWindowBackground,
        "underPageBackground" => NSVisualEffectMaterial::UnderPageBackground,
        other => return Err(format!("Unsupported window effect: {}", other)),
    };

    apply_vibrancy(
        &window,
        material,
        Some(NSVisualEffectState::FollowsWindowActiveState),
        None,
    )
    .map_err(|e| format!("Failed to apply vibrancy: {}", e))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn apply_window_effect_impl(
    _effect: &str,
    _app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    Ok(())
}
