pub mod border;
pub mod params;
pub mod renderer;

use crate::interface::controller::Color;
use crate::interface::effect::{Effect, EffectMetadata};
#[cfg(target_os = "windows")]
use crate::resource::screen::windows::ScreenSubscription;
use border::{BlackBorderMode, BlackBorderProcessor};
use inventory;
use params::SCREEN_PARAMS;
use renderer::{render_frame, CropRegion};
use std::cell::RefCell;
use std::time::Duration;

pub struct ScreenMirrorEffect {
    width: usize,
    height: usize,
    #[cfg(target_os = "windows")]
    screen: Option<ScreenSubscription>,
    #[cfg(target_os = "windows")]
    display_index: usize,
    smoothness: u32,
    auto_crop_enabled: bool,
    brightness: f32,
    saturation: f32,
    gamma: f32,
    black_border: RefCell<BlackBorderProcessor>,
    previous_buffer: Vec<Color>,
}

impl ScreenMirrorEffect {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            #[cfg(target_os = "windows")]
            screen: None,
            #[cfg(target_os = "windows")]
            display_index: 0,
            smoothness: 80,
            auto_crop_enabled: true,
            brightness: 1.0,
            saturation: 1.0,
            gamma: 1.0,
            black_border: RefCell::new(BlackBorderProcessor::new()),
            previous_buffer: Vec::new(),
        }
    }

    #[cfg(target_os = "windows")]
    fn ensure_subscription(&mut self) -> bool {
        if self.screen.is_none() {
            match ScreenSubscription::new(self.display_index) {
                Ok(handle) => {
                    self.screen = Some(handle);
                }
                Err(err) => {
                    eprintln!(
                        "[screen-mirror] Failed to init screen subscription ({}): {}",
                        self.display_index, err
                    );
                    self.screen = None;
                }
            }
        }

        self.screen.is_some()
    }

    fn paint_black(&self, buffer: &mut [Color]) {
        buffer.fill(Color::default());
    }

    fn capture_and_render(&mut self, buffer: &mut [Color]) -> bool {
        let layout = (self.width, self.height);

        if self.previous_buffer.len() != buffer.len() {
            self.previous_buffer.resize(buffer.len(), Color::default());
        }

        #[cfg(target_os = "windows")]
        {
            if !self.ensure_subscription() {
                return false;
            }

            let prev = &mut self.previous_buffer;
            let smoothness = self.smoothness;
            if let Some(subscription) = self.screen.as_mut() {
                let auto_crop_enabled = self.auto_crop_enabled;
                let black_border = &self.black_border;

                if !auto_crop_enabled {
                    // Ensure processor is reset when auto-crop is disabled.
                    black_border.borrow_mut().set_enabled(false);
                }

                match ScreenSubscription::capture_with(subscription, |frame| {
                    let crop = if auto_crop_enabled {
                        let mut processor = black_border.borrow_mut();
                        processor.set_enabled(true);
                        processor.process_frame(frame);
                        processor.crop_region_for(frame)
                    } else {
                        CropRegion::default()
                    };

                    render_frame(
                        layout,
                        frame,
                        buffer,
                        prev,
                        smoothness,
                        &crop,
                        self.brightness,
                        self.saturation,
                        self.gamma,
                    )
                }) {
                    Ok(true) => {
                        return true;
                    }
                    Ok(false) => {
                        // No active duplicator for this display yet.
                        return false;
                    }
                    Err(err) => {
                        eprintln!("[screen-mirror] capture error: {}", err);
                        // Drop current subscription so that a new one (and duplicator)
                        // will be created on the next tick if needed.
                        self.screen = None;
                        return false;
                    }
                }
            }

            return false;
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = layout;
            let _ = buffer;
            false
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
        if let Some(smoothness) = _params.get("smoothness").and_then(|v| v.as_f64()) {
            self.smoothness = smoothness.clamp(0.0, 100.0) as u32;
        }

        if let Some(auto_crop) = _params.get("autoCrop").and_then(|v| v.as_bool()) {
            self.auto_crop_enabled = auto_crop;
            #[cfg(target_os = "windows")]
            {
                self.black_border
                    .borrow_mut()
                    .set_enabled(self.auto_crop_enabled);
            }
        }

        if let Some(val) = _params.get("brightness").and_then(|v| v.as_f64()) {
            self.brightness = val as f32;
        }
        if let Some(val) = _params.get("saturation").and_then(|v| v.as_f64()) {
            self.saturation = val as f32;
        }
        if let Some(val) = _params.get("gamma").and_then(|v| v.as_f64()) {
            self.gamma = val as f32;
        }

        #[cfg(target_os = "windows")]
        {
            let mut bb = self.black_border.borrow_mut();

            if let Some(threshold) = _params.get("bbThreshold").and_then(|v| v.as_f64()) {
                bb.set_threshold_percent(threshold as f32);
            }

            if let Some(value) = _params
                .get("bbUnknownFrameCnt")
                .and_then(|v| v.as_f64())
            {
                bb.unknown_switch_cnt = value.max(0.0) as u32;
            }

            if let Some(value) = _params
                .get("bbBorderFrameCnt")
                .and_then(|v| v.as_f64())
            {
                bb.border_switch_cnt = value.max(0.0) as u32;
            }

            if let Some(value) = _params
                .get("bbMaxInconsistentCnt")
                .and_then(|v| v.as_f64())
            {
                bb.max_inconsistent_cnt = value.max(0.0) as u32;
            }

            if let Some(value) = _params
                .get("bbBlurRemoveCnt")
                .and_then(|v| v.as_f64())
            {
                bb.blur_remove_cnt = value.max(0.0) as i32;
            }

            if let Some(mode_value) = _params.get("bbMode").and_then(|v| v.as_f64()) {
                bb.mode = BlackBorderMode::from_value(mode_value as i32);
            }
        }

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
                    // Drop existing subscription so that the next capture will
                    // attach to the newly selected display via the manager.
                    self.screen = None;
                }
            }
        }
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
    icon: Some("Monitor"),
    params: &SCREEN_PARAMS,
    factory: factory,
});
