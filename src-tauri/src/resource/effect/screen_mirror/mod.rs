use crate::interface::controller::Color;
use crate::interface::effect::{Effect, EffectMetadata, EffectParam};
use crate::resource::screen::{ScreenCapturer, ScreenFrame};
use inventory;
use std::time::Duration;

#[cfg(target_os = "windows")]
use crate::resource::screen::DesktopDuplicator;

#[cfg(not(target_os = "windows"))]
use crate::resource::screen::windows::DesktopDuplicator;

const NO_PARAMS: &[EffectParam] = &[];

pub struct ScreenMirrorEffect {
    width: usize,
    height: usize,
    #[cfg(target_os = "windows")]
    capturer: Option<DesktopDuplicator>,
}

impl ScreenMirrorEffect {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            #[cfg(target_os = "windows")]
            capturer: None,
        }
    }

    #[cfg(target_os = "windows")]
    fn ensure_capturer(&mut self) -> Option<&mut DesktopDuplicator> {
        if self.capturer.is_none() {
            match DesktopDuplicator::new() {
                Ok(c) => self.capturer = Some(c),
                Err(err) => {
                    eprintln!("[screen-mirror] Failed to init capturer: {}", err);
                    return None;
                }
            }
        }
        self.capturer.as_mut()
    }

    #[cfg(not(target_os = "windows"))]
    fn ensure_capturer(&mut self) -> Option<&mut DesktopDuplicator> {
        None
    }

    fn paint_black(&self, buffer: &mut [Color]) {
        buffer.fill(Color::default());
    }

    fn capture_and_render(&mut self, buffer: &mut [Color]) -> bool {
        let layout = (self.width, self.height);
        match self.ensure_capturer() {
            Some(capturer) => match capturer.capture() {
                Ok(frame) => {
                    render_frame(layout, &frame, buffer);
                    true
                }
                Err(err) => {
                    eprintln!("[screen-mirror] capture error: {}", err);
                    #[cfg(target_os = "windows")]
                    {
                        self.capturer = None;
                    }
                    false
                }
            },
            None => false,
        }
    }
}

impl Effect for ScreenMirrorEffect {
    fn id(&self) -> String {
        "screen_mirror".to_string()
    }

    fn name(&self) -> String {
        "Screen Mirror".to_string()
    }

    fn tick(&mut self, _elapsed: Duration, buffer: &mut [Color]) {
        if buffer.is_empty() {
            return;
        }

        if self.capture_and_render(buffer) {
            return;
        }

        self.paint_black(buffer);
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn update_params(&mut self, _params: serde_json::Value) {
        // No configurable parameters in the simplified mode.
    }
}

fn render_frame(layout: (usize, usize), frame: &ScreenFrame<'_>, buffer: &mut [Color]) {
    if layout.1 <= 1 {
        render_linear(frame, buffer);
    } else {
        render_matrix(layout, frame, buffer);
    }
}

fn render_linear(frame: &ScreenFrame<'_>, buffer: &mut [Color]) {
    let leds = buffer.len();
    if leds == 0 {
        return;
    }

    for (index, color) in buffer.iter_mut().enumerate() {
        let ratio_x = if leds == 1 {
            0.5
        } else {
            (index as f32 + 0.5) / leds as f32
        };
        *color = sample_pixel(frame, ratio_x, 0.5);
    }
}

fn render_matrix(layout: (usize, usize), frame: &ScreenFrame<'_>, buffer: &mut [Color]) {
    let width = layout.0.max(1);
    let height = layout.1.max(1);
    let total = width.saturating_mul(height);
    let max_len = buffer.len().min(total);

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if idx >= max_len {
                return;
            }

            let ratio_x = if width == 1 {
                0.5
            } else {
                (x as f32 + 0.5) / width as f32
            };
            let ratio_y = if height == 1 {
                0.5
            } else {
                (y as f32 + 0.5) / height as f32
            };

            buffer[idx] = sample_pixel(frame, ratio_x, ratio_y);
        }
    }
}

fn sample_pixel(frame: &ScreenFrame<'_>, ratio_x: f32, ratio_y: f32) -> Color {
    let width = frame.width.max(1);
    let height = frame.height.max(1);

    let x = ((width - 1) as f32 * ratio_x.clamp(0.0, 1.0)).round() as u32;
    let y = ((height - 1) as f32 * ratio_y.clamp(0.0, 1.0)).round() as u32;

    let offset = (y as usize)
        .saturating_mul(frame.stride)
        .saturating_add(x as usize * 4);

    if offset + 3 >= frame.pixels.len() {
        return Color::default();
    }

    Color {
        r: frame.pixels[offset + 2],
        g: frame.pixels[offset + 1],
        b: frame.pixels[offset],
    }
}

fn factory() -> Box<dyn Effect> {
    Box::new(ScreenMirrorEffect::new())
}

inventory::submit!(EffectMetadata {
    id: "screen_mirror",
    name: "Screen Mirror",
    description: Some("Mirror the desktop colors onto matrices or strips"),
    group: Some("Screen Sync"),
    params: NO_PARAMS,
    factory: factory,
});

