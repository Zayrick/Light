use tauri::State;
use crate::manager::{Device, LightingManager};
use crate::manager::inventory::list_effects;
use crate::api::dto::{AppConfigDto, EffectInfo, EffectParamInfo, SystemInfoResponse};
use crate::api::config_store;
use crate::manager::PersistedDeviceConfig;

#[cfg(any(target_os = "windows", target_os = "macos"))]
use once_cell::sync::Lazy;
#[cfg(any(target_os = "windows", target_os = "macos"))]
use std::sync::Mutex;

use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(any(target_os = "windows", target_os = "macos"))]
use tauri::Manager;

use crate::resource::screen::{
    get_capture_fps as get_screen_capture_fps,
    get_capture_method as get_screen_capture_method,
    get_capture_max_pixels as get_screen_capture_max_pixels,
    list_displays as list_screen_displays,
    set_capture_fps as set_screen_capture_fps,
    set_capture_method as set_screen_capture_method,
    set_capture_max_pixels as set_screen_capture_max_pixels,
    normalize_capture_max_pixels,
    CaptureMethod,
    DisplayInfo,
    ScreenSubscription,
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

// ============================================================================
// App UI Settings (runtime)
// ============================================================================

static MINIMIZE_TO_TRAY: AtomicBool = AtomicBool::new(false);

pub fn minimize_to_tray_enabled() -> bool {
    MINIMIZE_TO_TRAY.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn get_minimize_to_tray() -> bool {
    MINIMIZE_TO_TRAY.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_minimize_to_tray(enabled: bool, app_handle: tauri::AppHandle) {
    MINIMIZE_TO_TRAY.store(enabled, Ordering::Relaxed);
    save_runtime_app_config_best_effort(&app_handle);
}

// ============================================================================
// Persisted App Config (app.json)
// ============================================================================

pub fn default_window_effect_for_platform() -> &'static str {
    default_effect_for_platform()
}

fn runtime_app_config_snapshot(app_handle: &tauri::AppHandle) -> AppConfigDto {
    let capture_method = get_capture_method();
    let window_effect = get_window_effect();

    let mut cfg = AppConfigDto::default_for_platform();
    cfg.window_effect = window_effect;
    cfg.minimize_to_tray = get_minimize_to_tray();
    cfg.screen_capture.max_pixels = get_screen_capture_max_pixels();
    cfg.screen_capture.fps = get_capture_fps();
    cfg.screen_capture.method = capture_method;

    // Ensure platform default effect is never persisted as empty string.
    if cfg.window_effect.is_empty() {
        cfg.window_effect = default_effect_for_platform().to_string();
    }

    let _ = app_handle;
    cfg
}

pub fn apply_app_config_to_runtime(cfg: &AppConfigDto, app_handle: &tauri::AppHandle) {
    // Minimize-to-tray
    MINIMIZE_TO_TRAY.store(cfg.minimize_to_tray, Ordering::Relaxed);

    // Screen capture
    set_screen_capture_max_pixels(cfg.screen_capture.max_pixels);
    set_screen_capture_fps(cfg.screen_capture.fps);
    if let Ok(requested) = cfg.screen_capture.method.parse::<CaptureMethod>() {
        set_screen_capture_method(requested);

        // Best-effort warm-up: triggers fallback early so we can persist the effective backend.
        #[cfg(target_os = "windows")]
        {
            let output_index = list_screen_displays()
                .ok()
                .and_then(|d| d.first().map(|x| x.index))
                .unwrap_or(0);

            let _ = ScreenSubscription::new(output_index);

            let effective = get_screen_capture_method();
            if effective != requested {
                // Persist the effective method so the frontend/config stay in sync.
                save_runtime_app_config_best_effort(app_handle);
            }
        }
    }

    // Window effect
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let effect = if cfg.window_effect.is_empty() {
            default_effect_for_platform()
        } else {
            cfg.window_effect.as_str()
        };

        if let Err(err) = apply_window_effect_impl(effect, app_handle) {
            log::warn!(effect, err:display = err; "[window_effect] Failed to apply persisted window effect");
        } else {
            let mut guard = CURRENT_WINDOW_EFFECT.lock().unwrap();
            *guard = effect.to_string();
        }
    }
}

fn save_runtime_app_config_best_effort(app_handle: &tauri::AppHandle) {
    let cfg = runtime_app_config_snapshot(app_handle);
    if let Err(err) = config_store::save_app_config(app_handle, &cfg) {
        log::warn!(err:display = err; "[config] Failed to persist app config");
    }
}

#[tauri::command]
pub fn get_app_config(app_handle: tauri::AppHandle) -> AppConfigDto {
    match config_store::load_app_config(&app_handle) {
        Ok(mut cfg) => {
            // Normalize empty window_effect to runtime/platform default.
            if cfg.window_effect.is_empty() {
                cfg.window_effect = get_window_effect();
            }
            cfg
        }
        Err(err) => {
            log::warn!(err:display = err; "[config] Failed to load app config; using runtime snapshot");
            runtime_app_config_snapshot(&app_handle)
        }
    }
}

#[tauri::command]
pub fn set_app_config(config: AppConfigDto, app_handle: tauri::AppHandle) -> Result<AppConfigDto, String> {
    let mut cfg = config;

    // Clamp numeric values defensively.
    cfg.screen_capture.max_pixels = normalize_capture_max_pixels(cfg.screen_capture.max_pixels);
    cfg.screen_capture.fps = cfg.screen_capture.fps.clamp(1, 60);

    // Normalize windowEffect.
    if cfg.window_effect.is_empty() {
        cfg.window_effect = default_effect_for_platform().to_string();
    }

    apply_app_config_to_runtime(&cfg, &app_handle);

    // Ensure we persist/return the effective capture method after warm-up/fallback.
    // This keeps the UI, runtime, and on-disk config aligned.
    let effective_method = get_screen_capture_method().to_string();
    cfg.screen_capture.method = effective_method;

    // Persist.
    config_store::save_app_config(&app_handle, &cfg)?;
    Ok(cfg)
}

// ============================================================================
// Persisted Device Config (devices/<id>.json)
// ============================================================================

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceConfigResponse {
    pub device_id: String,
    pub port: String,
    pub config: Option<PersistedDeviceConfig>,
}

fn save_device_config_best_effort(
    manager: &LightingManager,
    port: &str,
    app_handle: &tauri::AppHandle,
) {
    match manager.export_persisted_device_config(port) {
        Ok((device_id, cfg)) => {
            if let Err(err) = config_store::save_device_config(app_handle, &device_id, &cfg) {
                log::warn!(port, device_id = device_id.as_str(), err:display = err; "[config] Failed to persist device config");
            }
        }
        Err(err) => {
            log::warn!(port, err:display = err; "[config] Failed to export device config");
        }
    }
}

#[tauri::command]
pub fn get_device_config(
    port: String,
    manager: State<'_, LightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<DeviceConfigResponse, String> {
    let device = manager.get_device(&port)?;
    let cfg = config_store::load_device_config(&app_handle, &device.id)
        .map_err(|e| format!("Failed to load device config: {e}"))?;

    Ok(DeviceConfigResponse {
        device_id: device.id,
        port,
        config: cfg,
    })
}

#[tauri::command]
pub async fn scan_devices(
    manager: State<'_, LightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<Device>, String> {
    // 1) Probe hardware.
    let _ = manager.scan_devices();

    // 2) Restore per-device persisted configs (best-effort) and start runners if needed.
    let devices = manager.get_devices();
    for d in &devices {
        match config_store::load_device_config(&app_handle, &d.id) {
            Ok(Some(persisted)) => {
                if let Err(err) = manager.apply_persisted_device_config(&d.port, &persisted, app_handle.clone()) {
                    log::warn!(port = d.port.as_str(), device_id = d.id.as_str(), err:display = err; "[config] Failed to apply persisted device config");
                }
            }
            Ok(None) => {}
            Err(err) => {
                log::warn!(port = d.port.as_str(), device_id = d.id.as_str(), err:display = err; "[config] Failed to load persisted device config");
            }
        }
    }

    Ok(manager.get_devices())
}

#[tauri::command]
pub fn get_devices(manager: State<'_, LightingManager>) -> Result<Vec<Device>, String> {
    Ok(manager.get_devices())
}

#[tauri::command]
pub fn get_device(port: String, manager: State<'_, LightingManager>) -> Result<Device, String> {
    manager.get_device(&port)
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
            log::error!(err:display = err; "[screen] Failed to enumerate displays");
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
    manager.set_scope_effect_wait_ready(
        &port,
        None,
        None,
        Some(&effect_id),
        app_handle.clone(),
    )?;

    save_device_config_best_effort(&manager, &port, &app_handle);
    Ok(())
}

#[tauri::command]
pub fn update_effect_params(
    port: String,
    params: serde_json::Value,
    manager: State<LightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    manager.update_scope_effect_params(&port, None, None, params)?;
    save_device_config_best_effort(&manager, &port, &app_handle);
    Ok(())
}

#[tauri::command]
pub fn set_scope_effect(
    port: String,
    output_id: Option<String>,
    segment_id: Option<String>,
    effect_id: Option<String>,
    manager: State<LightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    manager.set_scope_effect_wait_ready(
        &port,
        output_id.as_deref(),
        segment_id.as_deref(),
        effect_id.as_deref(),
        app_handle.clone(),
    )?;

    save_device_config_best_effort(&manager, &port, &app_handle);
    Ok(())
}

#[tauri::command]
pub fn update_scope_effect_params(
    port: String,
    output_id: Option<String>,
    segment_id: Option<String>,
    params: serde_json::Value,
    manager: State<LightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    manager.update_scope_effect_params(
        &port,
        output_id.as_deref(),
        segment_id.as_deref(),
        params,
    )?;

    save_device_config_best_effort(&manager, &port, &app_handle);
    Ok(())
}

#[tauri::command]
pub fn set_output_segments(
    port: String,
    output_id: String,
    segments: Vec<crate::interface::controller::SegmentDefinition>,
    manager: State<LightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    manager.set_output_segments(&port, &output_id, segments)?;
    save_device_config_best_effort(&manager, &port, &app_handle);
    Ok(())
}

#[tauri::command]
pub fn set_brightness(
    port: String,
    brightness: u8,
    manager: State<LightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    manager.set_brightness(&port, brightness)?;
    save_device_config_best_effort(&manager, &port, &app_handle);
    Ok(())
}

#[tauri::command]
pub fn set_scope_brightness(
    port: String,
    output_id: Option<String>,
    segment_id: Option<String>,
    brightness: u8,
    manager: State<LightingManager>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    manager.set_scope_brightness(&port, output_id.as_deref(), segment_id.as_deref(), brightness)?;
    save_device_config_best_effort(&manager, &port, &app_handle);
    Ok(())
}

#[tauri::command]
pub fn set_capture_max_pixels(max_pixels: u32, app_handle: tauri::AppHandle) {
    set_screen_capture_max_pixels(max_pixels);
    save_runtime_app_config_best_effort(&app_handle);
}

#[tauri::command]
pub fn get_capture_max_pixels() -> u32 {
    get_screen_capture_max_pixels()
}

#[tauri::command]
pub fn set_capture_fps(fps: u8, app_handle: tauri::AppHandle) {
    set_screen_capture_fps(fps);
    save_runtime_app_config_best_effort(&app_handle);
}

#[tauri::command]
pub fn get_capture_fps() -> u8 {
    get_screen_capture_fps()
}

#[tauri::command]
pub fn set_capture_method(method: String, app_handle: tauri::AppHandle) {
    if let Ok(requested) = method.parse::<CaptureMethod>() {
        set_screen_capture_method(requested);

        #[cfg(target_os = "windows")]
        {
            let output_index = list_screen_displays()
                .ok()
                .and_then(|d| d.first().map(|x| x.index))
                .unwrap_or(0);
            let _ = ScreenSubscription::new(output_index);
        }
    }
    save_runtime_app_config_best_effort(&app_handle);
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

        save_runtime_app_config_best_effort(&app_handle);
        Ok(())
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
    let default = default_effect_for_platform().to_string();
    initialize_window_effect_with(app, &default);
}

// Used from lib.rs during app setup (restoring persisted effect)
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn initialize_window_effect_with(app: &tauri::App, effect: &str) {
    let handle = app.handle();
    let effective = if effect.is_empty() {
        default_effect_for_platform()
    } else {
        effect
    };

    if let Err(err) = apply_window_effect_impl(effective, handle) {
        log::warn!(
            effect = effective,
            err:display = err;
            "[window_effect] Failed to apply window effect during setup"
        );
    }

    let mut guard = CURRENT_WINDOW_EFFECT.lock().unwrap();
    *guard = effective.to_string();
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
        .unwrap_or(if build >= 22000 { 11 } else { 10 });

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
        log::warn!(err:display = err; "[window_effect] clear_mica failed");
    }
    if let Err(err) = clear_tabbed(&window) {
        log::warn!(err:display = err; "[window_effect] clear_tabbed failed");
    }
    if let Err(err) = clear_blur(&window) {
        log::warn!(err:display = err; "[window_effect] clear_blur failed");
    }
    if let Err(err) = clear_acrylic(&window) {
        log::warn!(err:display = err; "[window_effect] clear_acrylic failed");
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
        log::warn!(err:display = err; "[window_effect] clear_vibrancy failed");
    }

    // NOTE: Some legacy effect ids map to deprecated NSVisualEffectMaterial variants.
    // We keep them for compatibility and silence the warnings locally; consider
    // migrating callers to semantic materials (e.g., WindowBackground/ContentBackground).
    #[allow(deprecated)]
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
