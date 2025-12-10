use crate::interface::controller::{Controller, ControllerMetadata};
use crate::interface::effect::{Effect, EffectMetadata, EffectParamKind};
use serde_json::{Map, Value};

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

pub fn list_effects() -> Vec<&'static EffectMetadata> {
    inventory::iter::<EffectMetadata>.into_iter().collect()
}

pub fn get_effect_metadata(id: &str) -> Option<&'static EffectMetadata> {
    for effect in inventory::iter::<EffectMetadata> {
        if effect.id == id {
            return Some(effect);
        }
    }
    None
}

pub fn default_params_for_effect(id: &str) -> Option<Map<String, Value>> {
    let meta = get_effect_metadata(id)?;
    let mut map = Map::new();

    for param in meta.params {
        let value = match &param.kind {
            EffectParamKind::Slider { default, .. } => Value::from(*default),
            EffectParamKind::Select { default, .. } => Value::from(*default),
            EffectParamKind::Toggle { default } => Value::from(*default),
        };
        map.insert(param.key.to_string(), value);
    }

    Some(map)
}

pub fn create_effect(id: &str) -> Option<Box<dyn Effect>> {
    for effect in inventory::iter::<EffectMetadata> {
        if effect.id == id {
            return Some((effect.factory)());
        }
    }
    None
}
