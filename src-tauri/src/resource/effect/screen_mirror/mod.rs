use crate::interface::controller::Color;
use crate::interface::effect::{
    Effect, EffectMetadata, EffectParam, EffectParamKind, SelectOption, SelectOptions,
};
use crate::resource::screen::{ScreenCapturer, ScreenFrame};
use inventory;
use std::time::Duration;

#[cfg(target_os = "windows")]
use crate::resource::screen::DesktopDuplicator;

#[cfg(not(target_os = "windows"))]
use crate::resource::screen::windows::DesktopDuplicator;

const SCREEN_PARAMS: [EffectParam; 1] = [EffectParam {
    key: "displayIndex",
    label: "屏幕来源",
    kind: EffectParamKind::Select {
        default: 0.0,
        options: SelectOptions::Dynamic(screen_source_options),
    },
}];

#[cfg(target_os = "windows")]
fn screen_source_options() -> Result<Vec<SelectOption>, String> {
    use crate::resource::screen::windows::list_displays;

    list_displays()
        .map(|displays| {
            displays
                .into_iter()
                .map(|display| SelectOption {
                    label: format!("{} ({}x{})", display.name, display.width, display.height),
                    value: display.index as f64,
                })
                .collect()
        })
        .map_err(|err| err.to_string())
}

#[cfg(not(target_os = "windows"))]
fn screen_source_options() -> Result<Vec<SelectOption>, String> {
    Ok(Vec::new())
}

pub struct ScreenMirrorEffect {
    width: usize,
    height: usize,
    #[cfg(target_os = "windows")]
    capturer: Option<DesktopDuplicator>,
    #[cfg(target_os = "windows")]
    display_index: usize,
}

impl ScreenMirrorEffect {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            #[cfg(target_os = "windows")]
            capturer: None,
            #[cfg(target_os = "windows")]
            display_index: 0,
        }
    }

    #[cfg(target_os = "windows")]
    fn ensure_capturer(&mut self) -> Option<&mut DesktopDuplicator> {
        let mut needs_refresh = false;

        if let Some(capturer) = self.capturer.as_mut() {
            if capturer.output_index() != self.display_index {
                if let Err(err) = capturer.set_output_index(self.display_index) {
                    eprintln!(
                        "[screen-mirror] Failed to switch display ({}): {}",
                        self.display_index, err
                    );
                    self.capturer = None;
                    needs_refresh = true;
                }
            }
        } else {
            needs_refresh = true;
        }

        if needs_refresh {
            match DesktopDuplicator::with_output(self.display_index) {
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
        #[cfg(not(target_os = "windows"))]
        let _ = _params;

        #[cfg(target_os = "windows")]
        {
            if let Some(display_index_value) =
                _params.get("displayIndex").and_then(|value| value.as_u64())
            {
                let idx = display_index_value as usize;
                if idx != self.display_index {
                    self.display_index = idx;
                    if let Some(capturer) = self.capturer.as_mut() {
                        if let Err(err) = capturer.set_output_index(idx) {
                            eprintln!(
                                "[screen-mirror] Failed to apply display selection ({}): {}",
                                idx, err
                            );
                            self.capturer = None;
                        }
                    }
                }
            }
        }
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
    params: &SCREEN_PARAMS,
    factory: factory,
});
