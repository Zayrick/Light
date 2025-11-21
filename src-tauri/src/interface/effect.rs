use super::controller::Color;
use std::time::Duration;

pub trait Effect: Send {
    fn name(&self) -> String;
    fn tick(&mut self, elapsed: Duration, led_count: usize) -> Vec<Color>;
}

pub struct EffectMetadata {
    pub name: &'static str,
    pub factory: fn() -> Box<dyn Effect>,
}

inventory::collect!(EffectMetadata);
