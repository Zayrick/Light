use crate::interface::effect::{Effect, EffectMetadata};
use crate::interface::controller::Color;
use std::time::Duration;
use serde_json::Value;
use inventory;

pub struct RainbowEffect {
    speed: f32,
}

impl Effect for RainbowEffect {
    fn id(&self) -> String {
        "rainbow".to_string()
    }

    fn name(&self) -> String {
        "Rainbow".to_string()
    }

    fn tick(&mut self, elapsed: Duration, buffer: &mut [Color]) {
        let led_count = buffer.len();
        // Simple animation logic: offset hue by time
        let offset = (elapsed.as_millis() as f32 * self.speed / 10.0) % 360.0; 

        for i in 0..led_count {
             let hue = ((i as f32 * 360.0 / led_count as f32) + offset) % 360.0;
             let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
             buffer[i] = Color { r, g, b };
        }
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
    Box::new(RainbowEffect { speed: 1.0 })
}

inventory::submit!(EffectMetadata {
    id: "rainbow",
    name: "Rainbow",
    factory: factory,
});
