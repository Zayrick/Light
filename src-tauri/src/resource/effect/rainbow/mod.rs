use crate::interface::controller::Color;
use crate::interface::effect::{Effect, EffectMetadata, EffectParam, EffectParamKind};
use inventory;
use serde_json::Value;
use std::time::Duration;

pub struct RainbowEffect {
    speed: f32,
    width: usize,
    height: usize,
}

const RAINBOW_PARAMS: [EffectParam; 1] = [EffectParam {
    key: "speed",
    label: "速度",
    kind: EffectParamKind::Slider {
        min: 0.0,
        max: 5.0,
        step: 0.1,
        default: 2.5,
    },
}];

impl Effect for RainbowEffect {
    fn id(&self) -> String {
        "rainbow".to_string()
    }

    fn name(&self) -> String {
        "Rainbow".to_string()
    }

    fn tick(&mut self, elapsed: Duration, buffer: &mut [Color]) {
        let led_count = buffer.len();
        if led_count == 0 {
            return;
        }

        // Use stored layout if available; fall back to a 1D line.
        let width = if self.width == 0 {
            led_count
        } else {
            self.width
        };
        let height = if self.height == 0 { 1 } else { self.height };

        // Simple animation logic: horizontal rainbow that scrolls over time,
        // with a slight vertical phase so matrix layout is obvious.
        let offset = (elapsed.as_millis() as f32 * self.speed / 10.0) % 360.0;

        let mut i = 0;
        for y in 0..height {
            for x in 0..width {
                if i >= led_count {
                    break;
                }
                let base = (x as f32 * 360.0 / width as f32) + offset;
                let hue = (base + (y as f32 * 20.0)) % 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
                buffer[i] = Color { r, g, b };
                i += 1;
            }
        }
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn update_params(&mut self, params: Value) {
        if let Some(speed) = params.get("speed").and_then(|v| v.as_f64()) {
            self.speed = speed as f32;
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

fn factory() -> Box<dyn Effect> {
    Box::new(RainbowEffect {
        speed: 1.0,
        width: 0,
        height: 0,
    })
}

inventory::submit!(EffectMetadata {
    id: "rainbow",
    name: "Rainbow",
    description: Some("Cycling rainbow colors"),
    group: Some("Dynamic"),
    params: &RAINBOW_PARAMS,
    factory: factory,
});
