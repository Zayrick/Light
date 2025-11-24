use super::controller::Color;
use std::time::Duration;
use serde_json::Value;

pub trait Effect: Send {
    fn id(&self) -> String;
    fn name(&self) -> String;
    fn tick(&mut self, elapsed: Duration, buffer: &mut [Color]);
    /// Called when the virtual device layout (width/height) changes.
    /// Default implementation ignores the size, which is fine for 1D effects.
    fn resize(&mut self, _width: usize, _height: usize) {}
    fn update_params(&mut self, _params: Value) {}
}

#[derive(Clone, Copy)]
pub enum EffectParamKind {
    Slider,
}

pub struct EffectParam {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: EffectParamKind,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: f64,
}

pub struct EffectMetadata {
    pub id: &'static str,
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub group: Option<&'static str>,
    pub params: &'static [EffectParam],
    pub factory: fn() -> Box<dyn Effect>,
}

inventory::collect!(EffectMetadata);
