use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::interface::controller::{Controller, ControllerMetadata, Color};
use crate::interface::effect::{Effect, EffectMetadata};

// --- Helper Functions (Inventory) ---

pub fn list_controller_drivers() -> Vec<&'static ControllerMetadata> {
    inventory::iter::<ControllerMetadata>.into_iter().collect()
}

pub fn scan_controllers() -> Vec<Box<dyn Controller>> {
    let mut controllers = Vec::new();
    for driver in inventory::iter::<ControllerMetadata> {
        println!("Probing driver: {}", driver.name);
        controllers.extend((driver.probe)());
    }
    controllers
}

pub fn list_effects() -> Vec<&'static str> {
    inventory::iter::<EffectMetadata>.into_iter().map(|e| e.name).collect()
}

pub fn create_effect(name: &str) -> Option<Box<dyn Effect>> {
    for effect in inventory::iter::<EffectMetadata> {
        if effect.name == name {
            return Some((effect.factory)());
        }
    }
    None
}

// --- LightingManager (Business Logic) ---

#[derive(serde::Serialize, Clone)]
pub struct Device {
    pub port: String,
    pub model: String,
    pub id: String,
}

pub struct LightingManager {
    controllers: Mutex<HashMap<String, Arc<Mutex<Box<dyn Controller>>>>>,
    effect_flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl LightingManager {
    pub fn new() -> Self {
        Self {
            controllers: Mutex::new(HashMap::new()),
            effect_flags: Mutex::new(HashMap::new()),
        }
    }

    pub fn scan_devices(&self) -> Vec<Device> {
        let found_controllers = scan_controllers();
        
        let mut state_controllers = self.controllers.lock().unwrap();
        for controller in found_controllers {
            let port = controller.port_name();
            // Only add if not already present
            if !state_controllers.contains_key(&port) {
                state_controllers.insert(port, Arc::new(Mutex::new(controller)));
            }
        }

        let mut devices = Vec::new();
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

    pub fn start_effect(&self, port: &str, effect_name: &str) -> Result<(), String> {
        let controller_arc = {
            let controllers = self.controllers.lock().unwrap();
            controllers.get(port).cloned()
        };

        let controller_arc = match controller_arc {
            Some(c) => c,
            None => return Err("Device not found".to_string()),
        };

        // Stop existing effect first
        self.stop_effect_flag(port);

        // Set new flag
        let flag = Arc::new(AtomicBool::new(true));
        {
            let mut flags = self.effect_flags.lock().unwrap();
            flags.insert(port.to_string(), flag.clone());
        }

        let c_clone = controller_arc.clone();
        let flag_clone = flag.clone();
        let effect_name = effect_name.to_string();

        thread::spawn(move || {
            let mut effect = match create_effect(&effect_name) {
                Some(e) => e,
                None => return,
            };

            let start = std::time::Instant::now();
            
            // Get LED count from controller
            let led_count = {
                let c = c_clone.lock().unwrap();
                c.length()
            };

            while flag_clone.load(Ordering::Relaxed) {
                let colors = effect.tick(start.elapsed(), led_count);
                
                {
                    let mut c = c_clone.lock().unwrap();
                    if let Err(_) = c.update(&colors) {
                        break; // Stop if update fails
                    }
                }
                thread::sleep(Duration::from_millis(16));
            }
        });

        Ok(())
    }

    fn stop_effect_flag(&self, port: &str) {
        let flags = self.effect_flags.lock().unwrap();
        if let Some(flag) = flags.get(port) {
            flag.store(false, Ordering::Relaxed);
        }
    }

    pub fn turn_off(&self, port: &str) -> Result<(), String> {
        self.stop_effect_flag(port);
        
        // Give the thread a moment to exit
        thread::sleep(Duration::from_millis(50));

        let controller_arc = {
            let controllers = self.controllers.lock().unwrap();
            controllers.get(port).cloned()
        };

        if let Some(c_arc) = controller_arc {
            let mut c = c_arc.lock().unwrap();
            let len = c.length();
            let black = vec![Color::default(); len];
            let _ = c.update(&black);
        }
        
        Ok(())
    }
}
