use super::controller::Color;
use serde_json::Value;
use std::time::Duration;

pub trait Effect: Send {
    fn id(&self) -> String;
    fn name(&self) -> String;
    fn tick(&mut self, elapsed: Duration, buffer: &mut [Color]);
    /// Called when the virtual device layout (width/height) changes.
    /// Default implementation ignores the size, which is fine for 1D effects.
    fn resize(&mut self, _width: usize, _height: usize) {}
    fn update_params(&mut self, _params: Value) {}
}

/// How the frontend should treat a parameter when its dependency condition is not met.
#[derive(Clone, Copy, Debug)]
pub enum DependencyBehavior {
    /// Completely hide the parameter from the UI.
    Hide,
    /// Keep the parameter visible but disabled (read‑only).
    Disable,
}

/// Declarative dependency between two parameters of the same effect.
///
/// This allows the backend to describe simple rules like:
/// - Only show slider B when select A has a certain value
/// - Disable advanced settings when a toggle is off
#[derive(Clone, Copy, Debug)]
pub enum EffectParamDependency {
    Dependency {
        /// Key of the parameter this one depends on.
        key: &'static str,
        /// Optional equality condition: dependency is met when `key` equals this value.
        equals: Option<f64>,
        /// Optional inequality condition: dependency is met when `key` does NOT equal this value.
        not_equals: Option<f64>,
        /// How the frontend should react when the dependency is not satisfied.
        behavior: DependencyBehavior,
    },
    Always(DependencyBehavior),
}

pub struct EffectParam {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: EffectParamKind,
    /// Optional dependency that describes when this parameter is active/visible.
    pub dependency: Option<EffectParamDependency>,
}

pub enum EffectParamKind {
    Slider {
        min: f64,
        max: f64,
        step: f64,
        default: f64,
    },
    Select {
        default: f64,
        options: SelectOptions,
    },
}

pub enum SelectOptions {
    Static(&'static [StaticSelectOption]),
    Dynamic(DynamicSelectOptions),
}

pub struct StaticSelectOption {
    pub label: &'static str,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct SelectOption {
    pub label: String,
    pub value: f64,
}

pub type DynamicSelectOptions = fn() -> Result<Vec<SelectOption>, String>;

impl SelectOptions {
    pub fn resolve(&self) -> Result<Vec<SelectOption>, String> {
        match self {
            SelectOptions::Static(options) => Ok(options
                .iter()
                .map(|option| SelectOption {
                    label: option.label.to_string(),
                    value: option.value,
                })
                .collect()),
            SelectOptions::Dynamic(loader) => loader(),
        }
    }
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
