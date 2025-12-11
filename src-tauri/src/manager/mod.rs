pub mod inventory;
pub mod runner;

use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

use self::inventory::{default_params_for_effect, scan_controllers};
use self::runner::EffectRunner;
use crate::interface::controller::{Controller, Zone};

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
    pub current_effect_params: Option<Map<String, Value>>,
}

type ControllerRef = Arc<Mutex<Box<dyn Controller>>>;

pub struct LightingManager {
    controllers: Mutex<HashMap<String, ControllerRef>>,
    active_effects: Mutex<HashMap<String, EffectRunner>>,
    device_brightness: Mutex<HashMap<String, u8>>,
    active_effect_ids: Mutex<HashMap<String, String>>,
    active_effect_params: Mutex<HashMap<String, Map<String, Value>>>,
}

impl Default for LightingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LightingManager {
    pub fn new() -> Self {
        Self {
            controllers: Mutex::new(HashMap::new()),
            active_effects: Mutex::new(HashMap::new()),
            device_brightness: Mutex::new(HashMap::new()),
            active_effect_ids: Mutex::new(HashMap::new()),
            active_effect_params: Mutex::new(HashMap::new()),
        }
    }

    pub fn scan_devices(&self) -> Vec<Device> {
        let found_controllers = scan_controllers();

        let mut state_controllers = self.controllers.lock().unwrap();
        for controller in found_controllers {
            let port = controller.port_name();
            // Only add if not already present
            state_controllers
                .entry(port)
                .or_insert_with(|| Arc::new(Mutex::new(controller)));
        }

        let mut devices = Vec::new();
        let brightness_map = self.device_brightness.lock().unwrap();
        let effect_map = self.active_effect_ids.lock().unwrap();
        let params_map = self.active_effect_params.lock().unwrap();

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
                current_effect_params: params_map.get(port).cloned(),
            });
        }
        devices
    }

    pub fn start_effect(
        &self,
        port: &str,
        effect_id: &str,
        app_handle: AppHandle,
    ) -> Result<(), String> {
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

        let default_params = default_params_for_effect(effect_id);
        let runner = EffectRunner::start(effect_id, controller_arc, app_handle, brightness)?;

        if let Some(defaults) = default_params.clone() {
            runner.update_params(Value::Object(defaults));
        }

        let mut active = self.active_effects.lock().unwrap();
        active.insert(port.to_string(), runner);

        // Track the active effect id for this device
        {
            let mut ids = self.active_effect_ids.lock().unwrap();
            ids.insert(port.to_string(), effect_id.to_string());
        }

        // Track default params for the active effect
        {
            let mut params = self.active_effect_params.lock().unwrap();
            if let Some(defaults) = default_params {
                params.insert(port.to_string(), defaults);
            } else {
                params.remove(port);
            }
        }

        Ok(())
    }

    pub fn update_effect_params(&self, port: &str, params: Value) -> Result<(), String> {
        let active = self.active_effects.lock().unwrap();
        if let Some(runner) = active.get(port) {
            runner.update_params(params.clone());
        } else {
            return Err("No active effect on this device".to_string());
        }

        drop(active);

        if let Some(obj) = params.as_object() {
            let mut stored = self.active_effect_params.lock().unwrap();
            let entry = stored.entry(port.to_string()).or_default();
            for (key, value) in obj {
                entry.insert(key.clone(), value.clone());
            }
        }
        Ok(())
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

        let mut params = self.active_effect_params.lock().unwrap();
        params.remove(port);
    }
}
