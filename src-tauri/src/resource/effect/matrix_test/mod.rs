use crate::interface::effect::{Effect, EffectMetadata};
use crate::interface::controller::Color;
use std::time::Duration;
use inventory;

/// Very visible matrix test pattern:
/// - Fills four quadrants with red/green/blue/white blocks
/// - Adds a moving white scan line so orientation is obvious.
pub struct MatrixTestEffect {
    width: usize,
    height: usize,
}

impl Effect for MatrixTestEffect {
    fn id(&self) -> String {
        "matrix_test".to_string()
    }

    fn name(&self) -> String {
        "Matrix Test".to_string()
    }

    fn tick(&mut self, elapsed: Duration, buffer: &mut [Color]) {
        let len = buffer.len();
        if len == 0 {
            return;
        }

        let width = if self.width == 0 { len } else { self.width };
        let height = if self.height == 0 { 1 } else { self.height };

        // Base quadrant colors
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                if idx >= len {
                    break;
                }

                let half_w = width / 2;
                let half_h = height / 2;

                let color = if y < half_h && x < half_w {
                    // Top-left: Red
                    Color { r: 255, g: 0, b: 0 }
                } else if y < half_h && x >= half_w {
                    // Top-right: Green
                    Color { r: 0, g: 255, b: 0 }
                } else if y >= half_h && x < half_w {
                    // Bottom-left: Blue
                    Color { r: 0, g: 0, b: 255 }
                } else {
                    // Bottom-right: White
                    Color { r: 255, g: 255, b: 255 }
                };

                buffer[idx] = color;
            }
        }

        // Add a bright horizontal scan line moving downwards.
        let t = (elapsed.as_millis() / 50) as usize;
        let line_y = if height > 0 { t % height } else { 0 };

        if height > 0 {
            for x in 0..width {
                let idx = line_y * width + x;
                if idx >= len {
                    break;
                }
                buffer[idx] = Color { r: 255, g: 255, b: 255 };
            }
        }
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }
}

fn factory() -> Box<dyn Effect> {
    Box::new(MatrixTestEffect { width: 0, height: 0 })
}

inventory::submit!(EffectMetadata {
    id: "matrix_test",
    name: "Matrix Test",
    factory: factory,
});


