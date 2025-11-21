use crate::interface::controller::{Controller, ControllerMetadata};
use crate::interface::effect::{Effect, EffectMetadata};

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

