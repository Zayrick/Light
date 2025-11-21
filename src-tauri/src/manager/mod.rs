pub mod inventory;
pub mod runner;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::interface::controller::Controller;
use self::inventory::scan_controllers;
use self::runner::EffectRunner;

#[derive(serde::Serialize, Clone)]
pub struct Device {
    pub port: String,
    pub model: String,
    pub id: String,
}

pub struct LightingManager {
    controllers: Mutex<HashMap<String, Arc<Mutex<Box<dyn Controller>>>>>,
    active_effects: Mutex<HashMap<String, EffectRunner>>,
}

impl LightingManager {
    pub fn new() -> Self {
        Self {
            controllers: Mutex::new(HashMap::new()),
            active_effects: Mutex::new(HashMap::new()),
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
        self.stop_active_effect(port);

        let runner = EffectRunner::start(effect_name, controller_arc)?;

        let mut active = self.active_effects.lock().unwrap();
        active.insert(port.to_string(), runner);

        Ok(())
    }

    fn stop_active_effect(&self, port: &str) {
        let mut active = self.active_effects.lock().unwrap();
        if let Some(runner) = active.remove(port) {
            runner.stop();
        }
    }
}
