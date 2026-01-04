use crate::interface::controller::Color;
use crate::interface::effect::{Effect, EffectMetadata, EffectParam, EffectParamKind};
use inventory;
use serde_json::Value;
use std::time::Duration;

const DEFAULT_COLOR: &str = "#ffffff";

pub struct MonochromeEffect {
    color: Color,
}

const MONOCHROME_PARAMS: [EffectParam; 1] = [EffectParam {
    key: "color",
    label: "Color",
    kind: EffectParamKind::Color {
        default: DEFAULT_COLOR,
    },
    dependency: None,
}];

impl Effect for MonochromeEffect {
    fn id(&self) -> String {
        "monochrome".to_string()
    }

    fn name(&self) -> String {
        "Monochrome".to_string()
    }

    fn tick(&mut self, _elapsed: Duration, buffer: &mut [Color]) {
        buffer.fill(self.color);
    }

    fn update_params(&mut self, params: Value) {
        if let Some(value) = params.get("color").and_then(|v| v.as_str()) {
            if let Some(color) = parse_color(value) {
                self.color = color;
            }
        }
    }
}

fn parse_color(value: &str) -> Option<Color> {
    parse_hex_color(value).or_else(|| parse_rgb_function(value))
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let mut hex = value.trim();
    if let Some(stripped) = hex.strip_prefix('#') {
        hex = stripped;
    }

    let hex = match hex.len() {
        8 => &hex[..6],
        _ => hex,
    };

    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color { r, g, b })
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Color { r, g, b })
        }
        _ => None,
    }
}

fn parse_rgb_function(value: &str) -> Option<Color> {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("rgb") {
        return None;
    }

    let open = trimmed.find('(')?;
    let close = trimmed.rfind(')')?;
    let inner = &trimmed[open + 1..close];
    let parts: Vec<&str> = inner.split(',').collect();
    if parts.len() < 3 {
        return None;
    }

    let parse_component = |raw: &str| -> Option<u8> {
        let value = raw.trim().parse::<f32>().ok()?;
        Some(value.round().clamp(0.0, 255.0) as u8)
    };

    Some(Color {
        r: parse_component(parts[0])?,
        g: parse_component(parts[1])?,
        b: parse_component(parts[2])?,
    })
}

fn factory() -> Box<dyn Effect> {
    let color = parse_hex_color(DEFAULT_COLOR).unwrap_or_default();
    Box::new(MonochromeEffect { color })
}

inventory::submit!(EffectMetadata {
    id: "monochrome",
    name: "Monochrome",
    description: Some("Solid color fill"),
    group: Some("Basic"),
    icon: Some("Palette"),
    params: &MONOCHROME_PARAMS,
    factory,
});
