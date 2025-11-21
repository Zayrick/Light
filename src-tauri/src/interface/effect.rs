use super::controller::Color;
use std::time::Duration;
use serde_json::Value;

pub trait Effect: Send {
    fn id(&self) -> String;
    fn name(&self) -> String;
    fn tick(&mut self, elapsed: Duration, buffer: &mut [Color]);
    fn update_params(&mut self, _params: Value) {}
}

pub struct EffectMetadata {
    pub id: &'static str,
    pub name: &'static str,
    pub factory: fn() -> Box<dyn Effect>,
}

inventory::collect!(EffectMetadata);
