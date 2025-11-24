pub mod inventory;
pub mod runner;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde_json::Value;
use tauri::AppHandle;

use crate::interface::controller::{Controller, Zone};
use self::inventory::scan_controllers;
use self::runner::EffectRunner;

#[derive(serde::Serialize, Clone)]
pub struct Device {
    pub port: String,
    pub model: String,
    pub description: String,
    pub id: String,
    pub length: usize,
    pub zones: Vec<Zone>,
    pub virtual_layout: (usize, usize),
    pub brightness: u8,
    pub current_effect_id: Option<String>,
}

pub struct LightingManager {
    controllers: Mutex<HashMap<String, Arc<Mutex<Box<dyn Controller>>>>>,
    active_effects: Mutex<HashMap<String, EffectRunner>>,
    device_brightness: Mutex<HashMap<String, u8>>,
    active_effect_ids: Mutex<HashMap<String, String>>,
}

impl LightingManager {
    pub fn new() -> Self {
        Self {
            controllers: Mutex::new(HashMap::new()),
            active_effects: Mutex::new(HashMap::new()),
            device_brightness: Mutex::new(HashMap::new()),
            active_effect_ids: Mutex::new(HashMap::new()),
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
        let brightness_map = self.device_brightness.lock().unwrap();
        let effect_map = self.active_effect_ids.lock().unwrap();

        for (port, c_arc) in state_controllers.iter() {
            let c = c_arc.lock().unwrap();
            let brightness = *brightness_map.get(port).unwrap_or(&100);
            let current_effect_id = effect_map.get(port).cloned();
            
            devices.push(Device {
                port: port.clone(),
                model: c.model(),
                description: c.description(),
                id: c.serial_id(),
                length: c.length(),
                zones: c.zones(),
                virtual_layout: c.virtual_layout(),
                brightness,
                current_effect_id,
            });
        }
        devices
    }

    pub fn start_effect(&self, port: &str, effect_id: &str, app_handle: AppHandle) -> Result<(), String> {
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

        let brightness = {
            let map = self.device_brightness.lock().unwrap();
            *map.get(port).unwrap_or(&100)
        };

        let runner = EffectRunner::start(effect_id, controller_arc, app_handle, brightness)?;

        let mut active = self.active_effects.lock().unwrap();
        active.insert(port.to_string(), runner);

        // Track the active effect id for this device
        {
            let mut ids = self.active_effect_ids.lock().unwrap();
            ids.insert(port.to_string(), effect_id.to_string());
        }

        Ok(())
    }

    pub fn update_effect_params(&self, port: &str, params: Value) -> Result<(), String> {
        let active = self.active_effects.lock().unwrap();
        if let Some(runner) = active.get(port) {
            runner.update_params(params);
            Ok(())
        } else {
            Err("No active effect on this device".to_string())
        }
    }

    pub fn set_brightness(&self, port: &str, brightness: u8) -> Result<(), String> {
        // Update stored brightness
        {
            let mut map = self.device_brightness.lock().unwrap();
            map.insert(port.to_string(), brightness);
        }

        // Update active effect if any
        let active = self.active_effects.lock().unwrap();
        if let Some(runner) = active.get(port) {
            runner.set_brightness(brightness);
        }
        Ok(())
    }

    fn stop_active_effect(&self, port: &str) {
        let mut active = self.active_effects.lock().unwrap();
        if let Some(runner) = active.remove(port) {
            runner.stop();
        }

        // Clear stored active effect id for this device
        let mut ids = self.active_effect_ids.lock().unwrap();
        ids.remove(port);
    }
}
