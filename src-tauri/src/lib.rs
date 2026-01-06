pub mod interface;
pub mod manager;
pub mod resource;
pub mod api;

use crate::manager::LightingManager;
use crate::api::commands;
use crate::api::config_store;
use log::LevelFilter;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy, WEBVIEW_TARGET};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let timezone_strategy = TimezoneStrategy::UseUtc;
    let max_level = if cfg!(debug_assertions) {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };

    let mut log_targets = Vec::new();
    if cfg!(debug_assertions) {
        log_targets.push(Target::new(TargetKind::Stdout));
        log_targets.push(Target::new(TargetKind::Webview));
    }

    // Split frontend(webview) / backend(rust) logs into separate files.
    log_targets.push(
        Target::new(TargetKind::LogDir {
            file_name: Some("webview".into()),
        })
        .filter(|metadata| metadata.target().starts_with(WEBVIEW_TARGET)),
    );
    log_targets.push(
        Target::new(TargetKind::LogDir {
            file_name: Some("rust".into()),
        })
        .filter(|metadata| !metadata.target().starts_with(WEBVIEW_TARGET)),
    );

    let log_plugin = tauri_plugin_log::Builder::new()
        .clear_targets()
        .targets(log_targets)
        .timezone_strategy(timezone_strategy.clone())
        .rotation_strategy(RotationStrategy::KeepSome(10))
        .max_file_size(5 * 1024 * 1024) // 5 MiB per file
        .level(max_level)
        // Enterprise-friendly structured logs (JSON per line).
        .format(move |out, message, record| {
            let mut obj = serde_json::Map::new();

            obj.insert(
                "ts".into(),
                serde_json::Value::String(timezone_strategy.get_now().to_string()),
            );
            obj.insert(
                "app".into(),
                serde_json::Value::String(env!("CARGO_PKG_NAME").to_string()),
            );
            obj.insert(
                "version".into(),
                serde_json::Value::String(env!("CARGO_PKG_VERSION").to_string()),
            );
            obj.insert(
                "pid".into(),
                serde_json::Value::Number(serde_json::Number::from(std::process::id())),
            );
            obj.insert(
                "level".into(),
                serde_json::Value::String(record.level().to_string()),
            );
            obj.insert(
                "target".into(),
                serde_json::Value::String(record.target().to_string()),
            );
            if let Some(module_path) = record.module_path() {
                obj.insert(
                    "module".into(),
                    serde_json::Value::String(module_path.to_string()),
                );
            }
            if let Some(file) = record.file() {
                obj.insert("file".into(), serde_json::Value::String(file.to_string()));
            }
            if let Some(line) = record.line() {
                obj.insert(
                    "line".into(),
                    serde_json::Value::Number(serde_json::Number::from(line)),
                );
            }
            if let Some(thread_name) = std::thread::current().name() {
                obj.insert(
                    "thread".into(),
                    serde_json::Value::String(thread_name.to_string()),
                );
            }

            // Extract structured key-values when present.
            let mut kv_obj = serde_json::Map::new();
            struct KvCollect<'a>(&'a mut serde_json::Map<String, serde_json::Value>);
            impl<'kvs, 'a> log::kv::VisitSource<'kvs> for KvCollect<'a> {
                fn visit_pair(
                    &mut self,
                    key: log::kv::Key<'kvs>,
                    value: log::kv::Value<'kvs>,
                ) -> Result<(), log::kv::Error> {
                    let v = if let Some(b) = value.to_bool() {
                        serde_json::Value::Bool(b)
                    } else if let Some(i) = value.to_i64() {
                        serde_json::Value::Number(serde_json::Number::from(i))
                    } else if let Some(u) = value.to_u64() {
                        serde_json::Value::Number(serde_json::Number::from(u))
                    } else if let Some(f) = value.to_f64() {
                        serde_json::Number::from_f64(f)
                            .map(serde_json::Value::Number)
                            .unwrap_or_else(|| serde_json::Value::String(value.to_string()))
                    } else if let Some(s) = value.to_borrowed_str() {
                        serde_json::Value::String(s.to_string())
                    } else {
                        serde_json::Value::String(value.to_string())
                    };

                    self.0.insert(key.as_str().to_string(), v);
                    Ok(())
                }
            }
            let mut visitor = KvCollect(&mut kv_obj);
            let _ = record.key_values().visit(&mut visitor);
            if !kv_obj.is_empty() {
                obj.insert("kv".into(), serde_json::Value::Object(kv_obj));
            }

            // If the message itself is JSON, embed it as an object/value.
            let msg = message.to_string();
            match serde_json::from_str::<serde_json::Value>(&msg) {
                Ok(v) => {
                    obj.insert("message".into(), v);
                }
                Err(_) => {
                    obj.insert("message".into(), serde_json::Value::String(msg));
                }
            }

            let line = serde_json::Value::Object(obj);
            match serde_json::to_string(&line) {
                Ok(json) => out.finish(format_args!("{json}")),
                Err(_) => out.finish(format_args!("[{}][{}] {}", record.level(), record.target(), message)),
            }
        })
        .build();

    tauri::Builder::default()
        .plugin(log_plugin)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .manage(LightingManager::new())
        .invoke_handler(tauri::generate_handler![
            commands::scan_devices,
            commands::get_devices,
            commands::get_device,
            commands::get_effects,
            commands::get_displays,
            commands::set_effect,
            commands::update_effect_params,
            commands::set_scope_effect,
            commands::update_scope_effect_params,
            commands::set_output_segments,
            commands::set_brightness,
            commands::set_scope_brightness,
            commands::set_capture_max_pixels,
            commands::get_capture_max_pixels,
            commands::set_capture_fps,
            commands::get_capture_fps,
            commands::set_capture_method,
            commands::get_capture_method,
            commands::get_window_effects,
            commands::get_window_effect,
            commands::set_window_effect,
            commands::get_system_info,
            commands::get_minimize_to_tray,
            commands::set_minimize_to_tray,
            commands::get_app_config,
            commands::set_app_config,
            commands::get_device_config,
        ])
        .on_window_event(|window, event| {
            // 只处理主窗口
            if window.label() != "main" {
                return;
            }

            // 当开启“最小化到托盘”时，将关闭请求改为 hide。
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if commands::minimize_to_tray_enabled() {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            // Log panics (best-effort) instead of silent crash.
            std::panic::set_hook(Box::new(|panic| {
                let payload = if let Some(s) = panic.payload().downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = panic.payload().downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic payload".to_string()
                };

                if let Some(location) = panic.location() {
                    log::error!(
                        panic:display = payload,
                        file = location.file(),
                        line = location.line(),
                        column = location.column();
                        "panic"
                    );
                } else {
                    log::error!(panic:display = payload; "panic");
                }
            }));

            log::info!("app starting");

            // Load persisted app config (best-effort) and apply it to runtime.
            // This must run before any UI queries so that `get_*` commands reflect persisted values.
            {
                let handle = app.handle();
                if let Ok(cfg) = config_store::load_app_config(handle) {
                    commands::apply_app_config_to_runtime(&cfg, handle);
                }
            }

            #[cfg(any(target_os = "windows", target_os = "macos"))]
            {
                // Prefer persisted `windowEffect` if available; otherwise fall back to platform default.
                let handle = app.handle();
                let effect = config_store::load_app_config(handle)
                    .ok()
                    .map(|c| c.window_effect)
                    .unwrap_or_else(|| commands::default_window_effect_for_platform().to_string());
                commands::initialize_window_effect_with(app, &effect);
            }

            // System tray (用于“最小化到托盘”以及快速恢复/退出)
            {
                use tauri::menu::{MenuBuilder, MenuItemBuilder};
                use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

                let show = MenuItemBuilder::with_id("show", "显示").build(app)?;
                let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
                let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

                let _tray = TrayIconBuilder::new()
                    .icon(app.default_window_icon().unwrap().clone())
                    .menu(&menu)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
