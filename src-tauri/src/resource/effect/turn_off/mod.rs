use crate::interface::effect::{Effect, EffectMetadata};
use crate::interface::controller::Color;
use std::time::Duration;
use inventory;

pub struct TurnOffEffect;

impl Effect for TurnOffEffect {
    fn id(&self) -> String {
        "turn_off".to_string()
    }

    fn name(&self) -> String {
        "Turn Off".to_string()
    }

    fn tick(&mut self, _elapsed: Duration, buffer: &mut [Color]) {
        buffer.fill(Color::default());
    }
}

fn factory() -> Box<dyn Effect> {
    Box::new(TurnOffEffect)
}

inventory::submit!(EffectMetadata {
    id: "turn_off",
    name: "Turn Off",
    factory: factory,
});

