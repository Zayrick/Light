pub mod interface;
pub mod manager;
pub mod resource;

use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use tauri::State;
use crate::interface::controller::Color;

#[derive(serde::Serialize, Clone)]
struct Device {
    port: String,
    model: String,
    id: String,
}

struct AppState {
    // Key: port name (unique ID)
    controllers: Mutex<HashMap<String, Arc<Mutex<Box<dyn interface::controller::Controller>>>>>,
    effect_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

#[tauri::command]
fn scan_devices(state: State<AppState>) -> Vec<Device> {
    // Scan for new controllers
    let found_controllers = manager::scan_controllers();
    
    let mut state_controllers = state.controllers.lock().unwrap();
    let mut devices = Vec::new();

    for controller in found_controllers {
        let port = controller.port_name();
        // Only add if not already present (to avoid dropping active connections)
        if !state_controllers.contains_key(&port) {
            state_controllers.insert(port.clone(), Arc::new(Mutex::new(controller)));
        }
    }

    // Return all known devices
    for (port, c_arc) in state_controllers.iter() {
        let c = c_arc.lock().unwrap();
        devices.push(Device {
            port: port.clone(),
            model: c.model(),
            id: c.serial_id(),
        });
    }
    
    devices
}

#[tauri::command]
fn set_rainbow(port: String, state: State<AppState>) -> Result<(), String> {
    let controller_arc = {
        let controllers = state.controllers.lock().unwrap();
        controllers.get(&port).cloned()
    };

    let controller_arc = match controller_arc {
        Some(c) => c,
        None => return Err("Device not found".to_string()),
    };

    // Stop existing effect
    {
        let mut flags = state.effect_flags.lock().unwrap();
        if let Some(flag) = flags.get(&port) {
            flag.store(false, Ordering::Relaxed);
        }
        
        let flag = Arc::new(AtomicBool::new(true));
        flags.insert(port.clone(), flag.clone());
        
        let c_clone = controller_arc.clone();
        let flag_clone = flag.clone();
        
        thread::spawn(move || {
            // Create Rainbow effect via manager
            let mut effect = match manager::create_effect("Rainbow") {
                Some(e) => e,
                None => return,
            };
            
            let start = std::time::Instant::now();
            
            while flag_clone.load(Ordering::Relaxed) {
                let colors = effect.tick(start.elapsed(), 100); // Assume 100 LEDs
                
                {
                    let mut c = c_clone.lock().unwrap();
                    if let Err(_) = c.update(&colors) {
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(16));
            }
        });
    }

    Ok(())
}

#[tauri::command]
fn turn_off(port: String, state: State<AppState>) -> Result<(), String> {
    // Stop effect
    {
        let flags = state.effect_flags.lock().unwrap();
        if let Some(flag) = flags.get(&port) {
            flag.store(false, Ordering::Relaxed);
        }
    }
    
    thread::sleep(Duration::from_millis(50));

    let controller_arc = {
        let controllers = state.controllers.lock().unwrap();
        controllers.get(&port).cloned()
    };

    if let Some(c_arc) = controller_arc {
        let mut c = c_arc.lock().unwrap();
        let black = vec![Color::default(); 100];
        let _ = c.update(&black);
    }
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            controllers: Mutex::new(HashMap::new()),
            effect_flags: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![scan_devices, set_rainbow, turn_off])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
