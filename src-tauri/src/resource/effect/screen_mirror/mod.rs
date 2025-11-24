use crate::interface::controller::Color;
use crate::interface::effect::{
    DependencyBehavior, Effect, EffectMetadata, EffectParam, EffectParamDependency,
    EffectParamKind, SelectOption, SelectOptions, StaticSelectOption,
};
use crate::resource::screen::ScreenFrame;
use inventory;
use std::cell::RefCell;
use std::time::Duration;

#[cfg(target_os = "windows")]
use crate::resource::screen::windows::ScreenSubscription;

const AUTO_CROP_OPTIONS: [StaticSelectOption; 2] = [
    StaticSelectOption {
        label: "禁用",
        value: 0.0,
    },
    StaticSelectOption {
        label: "自动黑边裁剪",
        value: 1.0,
    },
];

const BLACK_BORDER_MODE_OPTIONS: [StaticSelectOption; 4] = [
    StaticSelectOption {
        label: "默认模式",
        value: 0.0,
    },
    StaticSelectOption {
        label: "经典模式",
        value: 1.0,
    },
    StaticSelectOption {
        label: "OSD 模式",
        value: 2.0,
    },
    StaticSelectOption {
        label: "信箱模式",
        value: 3.0,
    },
];

const SCREEN_PARAMS: [EffectParam; 9] = [
    EffectParam {
        key: "displayIndex",
        label: "屏幕来源",
        kind: EffectParamKind::Select {
            default: 0.0,
            options: SelectOptions::Dynamic(screen_source_options),
        },
        dependency: None,
    },
    EffectParam {
        key: "smoothness",
        label: "平滑度",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            default: 80.0,
        },
        dependency: None,
    },
    EffectParam {
        key: "autoCrop",
        label: "黑边裁剪",
        kind: EffectParamKind::Select {
            default: 1.0,
            options: SelectOptions::Static(&AUTO_CROP_OPTIONS),
        },
        dependency: None,
    },
    EffectParam {
        key: "bbThreshold",
        label: "黑边判定阈值 (%)",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 100.0,
            step: 1.0,
            default: 5.0,
        },
        dependency: Some(EffectParamDependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
    EffectParam {
        key: "bbUnknownFrameCnt",
        label: "未知边框切换帧数",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 2000.0,
            step: 50.0,
            default: 600.0,
        },
        dependency: Some(EffectParamDependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
    EffectParam {
        key: "bbBorderFrameCnt",
        label: "稳定边框切换帧数",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 200.0,
            step: 1.0,
            default: 50.0,
        },
        dependency: Some(EffectParamDependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
    EffectParam {
        key: "bbMaxInconsistentCnt",
        label: "最大允许不一致帧数",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 50.0,
            step: 1.0,
            default: 10.0,
        },
        dependency: Some(EffectParamDependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
    EffectParam {
        key: "bbBlurRemoveCnt",
        label: "模糊安全边界 (像素)",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 10.0,
            step: 1.0,
            default: 1.0,
        },
        dependency: Some(EffectParamDependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
    EffectParam {
        key: "bbMode",
        label: "黑边检测模式",
        kind: EffectParamKind::Select {
            default: 0.0,
            options: SelectOptions::Static(&BLACK_BORDER_MODE_OPTIONS),
        },
        dependency: Some(EffectParamDependency {
            key: "autoCrop",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Disable,
        }),
    },
];

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
    screen: Option<ScreenSubscription>,
    #[cfg(target_os = "windows")]
    display_index: usize,
    smoothness: u32,
    auto_crop_enabled: bool,
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
            if let Some(subscription) = &self.screen {
                let auto_crop_enabled = self.auto_crop_enabled;
                let black_border = &self.black_border;

                if !auto_crop_enabled {
                    // Ensure processor is reset when auto-crop is disabled.
                    black_border.borrow_mut().set_enabled(false);
                }

                match subscription.capture_with(|frame| {
                    let crop = if auto_crop_enabled {
                        let mut processor = black_border.borrow_mut();
                        processor.set_enabled(true);
                        processor.process_frame(frame);
                        processor.crop_region_for(frame)
                    } else {
                        CropRegion::default()
                    };

                    render_frame(layout, frame, buffer, prev, smoothness, &crop)
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

        if let Some(auto_crop) = _params.get("autoCrop").and_then(|v| v.as_f64()) {
            self.auto_crop_enabled = auto_crop >= 0.5;
            #[cfg(target_os = "windows")]
            {
                self.black_border
                    .borrow_mut()
                    .set_enabled(self.auto_crop_enabled);
            }
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

fn render_frame(
    layout: (usize, usize),
    frame: &ScreenFrame<'_>,
    buffer: &mut [Color],
    previous_buffer: &mut [Color],
    smoothness: u32,
    crop: &CropRegion,
) {
    if layout.1 <= 1 {
        render_linear(frame, buffer, previous_buffer, smoothness, crop);
    } else {
        render_matrix(layout, frame, buffer, previous_buffer, smoothness, crop);
    }
}

fn interpolate(c1: Color, c2: Color, factor: f32) -> Color {
    Color {
        r: (c1.r as f32 + (c2.r as f32 - c1.r as f32) * factor) as u8,
        g: (c1.g as f32 + (c2.g as f32 - c1.g as f32) * factor) as u8,
        b: (c1.b as f32 + (c2.b as f32 - c1.b as f32) * factor) as u8,
    }
}

fn smooth_color(prev: Color, target: Color, smoothness: u32) -> Color {
    if smoothness == 0 {
        return target;
    }
    if smoothness >= 100 {
        return prev;
    }

    let factor = (100.0 - smoothness as f32) / 100.0;
    interpolate(prev, target, factor)
}

fn render_linear(
    frame: &ScreenFrame<'_>,
    buffer: &mut [Color],
    previous_buffer: &mut [Color],
    smoothness: u32,
    crop: &CropRegion,
) {
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
        let target = sample_pixel(frame, ratio_x, 0.5, crop);

        if index < previous_buffer.len() {
            let prev = previous_buffer[index];
            let smoothed = smooth_color(prev, target, smoothness);
            previous_buffer[index] = smoothed;
            *color = smoothed;
        } else {
            *color = target;
        }
    }
}

fn render_matrix(
    layout: (usize, usize),
    frame: &ScreenFrame<'_>,
    buffer: &mut [Color],
    previous_buffer: &mut [Color],
    smoothness: u32,
    crop: &CropRegion,
) {
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

            let target = sample_pixel(frame, ratio_x, ratio_y, crop);

            if idx < previous_buffer.len() {
                let prev = previous_buffer[idx];
                let smoothed = smooth_color(prev, target, smoothness);
                previous_buffer[idx] = smoothed;
                buffer[idx] = smoothed;
            } else {
                buffer[idx] = target;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct CropRegion {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

fn sample_pixel(frame: &ScreenFrame<'_>, ratio_x: f32, ratio_y: f32, crop: &CropRegion) -> Color {
    let width = frame.width.max(1);
    let height = frame.height.max(1);

    let crop_left = crop.left.clamp(0.0, 0.45);
    let crop_right = crop.right.clamp(0.0, 0.45);
    let crop_top = crop.top.clamp(0.0, 0.45);
    let crop_bottom = crop.bottom.clamp(0.0, 0.45);

    let roi_width = (1.0 - crop_left - crop_right).max(0.1);
    let roi_height = (1.0 - crop_top - crop_bottom).max(0.1);

    let rx = (crop_left + ratio_x.clamp(0.0, 1.0) * roi_width).clamp(0.0, 1.0);
    let ry = (crop_top + ratio_y.clamp(0.0, 1.0) * roi_height).clamp(0.0, 1.0);

    let x = ((width - 1) as f32 * rx).round() as u32;
    let y = ((height - 1) as f32 * ry).round() as u32;

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

#[derive(Clone, Copy, Debug, Default)]
struct BlackBorder {
    unknown: bool,
    horizontal_size: i32,
    vertical_size: i32,
}

impl PartialEq for BlackBorder {
    fn eq(&self, other: &Self) -> bool {
        if self.unknown {
            other.unknown
        } else {
            !other.unknown
                && self.horizontal_size == other.horizontal_size
                && self.vertical_size == other.vertical_size
        }
    }
}

impl Eq for BlackBorder {}

#[derive(Clone, Copy, Debug)]
enum BlackBorderMode {
    Default,
    Classic,
    Osd,
    Letterbox,
}

impl BlackBorderMode {
    fn from_value(value: i32) -> Self {
        match value {
            1 => BlackBorderMode::Classic,
            2 => BlackBorderMode::Osd,
            3 => BlackBorderMode::Letterbox,
            _ => BlackBorderMode::Default,
        }
    }
}

struct BlackBorderDetector {
    threshold: u8,
}

impl BlackBorderDetector {
    fn new(threshold_percent: f32) -> Self {
        Self {
            threshold: Self::calculate_threshold(threshold_percent),
        }
    }

    fn calculate_threshold(threshold_percent: f32) -> u8 {
        let rgb_threshold = (threshold_percent.clamp(0.0, 100.0) / 100.0 * 255.0).ceil() as i32;
        rgb_threshold.clamp(0, 255) as u8
    }

    #[inline]
    fn is_black_bgr(&self, b: u8, g: u8, r: u8) -> bool {
        b < self.threshold && g < self.threshold && r < self.threshold
    }

    #[inline]
    fn sample_bgr(frame: &ScreenFrame<'_>, x: i32, y: i32) -> Option<(u8, u8, u8)> {
        if x < 0 || y < 0 {
            return None;
        }

        let width = frame.width as i32;
        let height = frame.height as i32;
        if x >= width || y >= height {
            return None;
        }

        let offset = (y as usize)
            .saturating_mul(frame.stride)
            .saturating_add(x as usize * 4);
        if offset + 3 >= frame.pixels.len() {
            return None;
        }

        Some((
            frame.pixels[offset],
            frame.pixels[offset + 1],
            frame.pixels[offset + 2],
        ))
    }

    fn process_default(&self, frame: &ScreenFrame<'_>) -> BlackBorder {
        let mut width = frame.width as i32;
        let mut height = frame.height as i32;
        if width <= 0 || height <= 0 {
            return BlackBorder::default();
        }

        let width33 = width / 3;
        let height33 = height / 3;
        let width66 = width33 * 2;
        let height66 = height33 * 2;
        let x_center = width / 2;
        let y_center = height / 2;

        let mut first_non_black_x = -1;
        let mut first_non_black_y = -1;

        width -= 1;
        height -= 1;

        for x in 0..width33 {
            let coords = [
                (width - x, y_center),
                (x, height33),
                (x, height66),
            ];
            if coords.iter().any(|&(cx, cy)| {
                if let Some((b, g, r)) = Self::sample_bgr(frame, cx, cy) {
                    !self.is_black_bgr(b, g, r)
                } else {
                    false
                }
            }) {
                first_non_black_x = x;
                break;
            }
        }

        for y in 0..height33 {
            let coords = [
                (x_center, height - y),
                (width33, y),
                (width66, y),
            ];
            if coords.iter().any(|&(cx, cy)| {
                if let Some((b, g, r)) = Self::sample_bgr(frame, cx, cy) {
                    !self.is_black_bgr(b, g, r)
                } else {
                    false
                }
            }) {
                first_non_black_y = y;
                break;
            }
        }

        BlackBorder {
            unknown: first_non_black_x == -1 || first_non_black_y == -1,
            horizontal_size: first_non_black_y,
            vertical_size: first_non_black_x,
        }
    }

    fn process_classic(&self, frame: &ScreenFrame<'_>) -> BlackBorder {
        let width_third = (frame.width as i32) / 3;
        let height_third = (frame.height as i32) / 3;
        if width_third <= 0 || height_third <= 0 {
            return BlackBorder::default();
        }

        let max_size = width_third.max(height_third);

        let mut first_non_black_x = -1;
        let mut first_non_black_y = -1;

        for i in 0..max_size {
            let x = i.min(width_third);
            let y = i.min(height_third);

            if let Some((b, g, r)) = Self::sample_bgr(frame, x, y) {
                if !self.is_black_bgr(b, g, r) {
                    first_non_black_x = x;
                    first_non_black_y = y;
                    break;
                }
            }
        }

        if first_non_black_x >= 0 && first_non_black_y >= 0 {
            while first_non_black_x > 0 {
                let x = first_non_black_x - 1;
                if let Some((b, g, r)) = Self::sample_bgr(frame, x, first_non_black_y) {
                    if self.is_black_bgr(b, g, r) {
                        break;
                    }
                }
                first_non_black_x -= 1;
            }

            while first_non_black_y > 0 {
                let y = first_non_black_y - 1;
                if let Some((b, g, r)) = Self::sample_bgr(frame, first_non_black_x, y) {
                    if self.is_black_bgr(b, g, r) {
                        break;
                    }
                }
                first_non_black_y -= 1;
            }
        }

        BlackBorder {
            unknown: first_non_black_x == -1 || first_non_black_y == -1,
            horizontal_size: first_non_black_y,
            vertical_size: first_non_black_x,
        }
    }

    fn process_letterbox(&self, frame: &ScreenFrame<'_>) -> BlackBorder {
        let width = frame.width as i32;
        let mut height = frame.height as i32;
        if width <= 0 || height <= 0 {
            return BlackBorder::default();
        }

        let width25 = width / 4;
        let height33 = height / 3;
        let width75 = width25 * 3;
        let x_center = width / 2;

        let mut first_non_black_y = -1;

        height -= 1;

        for y in 0..height33 {
            let coords = [
                (x_center, y),
                (width25, y),
                (width75, y),
                (width25, height - y),
                (width75, height - y),
            ];
            if coords.iter().any(|&(cx, cy)| {
                if let Some((b, g, r)) = Self::sample_bgr(frame, cx, cy) {
                    !self.is_black_bgr(b, g, r)
                } else {
                    false
                }
            }) {
                first_non_black_y = y;
                break;
            }
        }

        BlackBorder {
            unknown: first_non_black_y == -1,
            horizontal_size: first_non_black_y,
            vertical_size: 0,
        }
    }

    fn process_osd(&self, frame: &ScreenFrame<'_>) -> BlackBorder {
        let mut width = frame.width as i32;
        let mut height = frame.height as i32;
        if width <= 0 || height <= 0 {
            return BlackBorder::default();
        }

        let width33 = width / 3;
        let height33 = height / 3;
        let height66 = height33 * 2;
        let y_center = height / 2;

        let mut first_non_black_x = -1;
        let mut first_non_black_y = -1;

        width -= 1;
        height -= 1;

        let mut x = 0;
        while x < width33 {
            let coords = [
                (width - x, y_center),
                (x, height33),
                (x, height66),
            ];
            if coords.iter().any(|&(cx, cy)| {
                if let Some((b, g, r)) = Self::sample_bgr(frame, cx, cy) {
                    !self.is_black_bgr(b, g, r)
                } else {
                    false
                }
            }) {
                first_non_black_x = x;
                break;
            }
            x += 1;
        }

        if first_non_black_x != -1 {
            for y in 0..height33 {
                let coords = [
                    (x, y),
                    (x, height - y),
                    (width - x, y),
                    (width - x, height - y),
                ];
                if coords.iter().any(|&(cx, cy)| {
                    if let Some((b, g, r)) = Self::sample_bgr(frame, cx, cy) {
                        !self.is_black_bgr(b, g, r)
                    } else {
                        false
                    }
                }) {
                    first_non_black_y = y;
                    break;
                }
            }
        }

        BlackBorder {
            unknown: first_non_black_x == -1 || first_non_black_y == -1,
            horizontal_size: first_non_black_y,
            vertical_size: first_non_black_x,
        }
    }

    fn detect(&self, frame: &ScreenFrame<'_>, mode: BlackBorderMode) -> BlackBorder {
        match mode {
            BlackBorderMode::Default => self.process_default(frame),
            BlackBorderMode::Classic => self.process_classic(frame),
            BlackBorderMode::Osd => self.process_osd(frame),
            BlackBorderMode::Letterbox => self.process_letterbox(frame),
        }
    }
}

struct BlackBorderProcessor {
    enabled: bool,
    unknown_switch_cnt: u32,
    border_switch_cnt: u32,
    max_inconsistent_cnt: u32,
    blur_remove_cnt: i32,
    mode: BlackBorderMode,
    detector: BlackBorderDetector,
    current_border: BlackBorder,
    previous_detected_border: BlackBorder,
    consistent_cnt: u32,
    inconsistent_cnt: u32,
}

impl BlackBorderProcessor {
    fn new() -> Self {
        Self {
            enabled: true,
            unknown_switch_cnt: 600,
            border_switch_cnt: 50,
            max_inconsistent_cnt: 10,
            blur_remove_cnt: 1,
            mode: BlackBorderMode::Default,
            detector: BlackBorderDetector::new(5.0),
            current_border: BlackBorder::default(),
            previous_detected_border: BlackBorder::default(),
            consistent_cnt: 0,
            inconsistent_cnt: 10,
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.reset_state();
        }
    }

    fn reset_state(&mut self) {
        self.current_border = BlackBorder::default();
        self.previous_detected_border = BlackBorder::default();
        self.consistent_cnt = 0;
        self.inconsistent_cnt = self.max_inconsistent_cnt;
    }

    fn set_threshold_percent(&mut self, threshold_percent: f32) {
        self.detector = BlackBorderDetector::new(threshold_percent);
    }

    fn update_border(&mut self, new_detected_border: BlackBorder) -> bool {
        if new_detected_border == self.previous_detected_border {
            self.consistent_cnt = self.consistent_cnt.saturating_add(1);
            self.inconsistent_cnt = 0;
        } else {
            self.inconsistent_cnt = self.inconsistent_cnt.saturating_add(1);
            if self.inconsistent_cnt <= self.max_inconsistent_cnt {
                return false;
            }
            self.previous_detected_border = new_detected_border;
            self.consistent_cnt = 0;
        }

        if self.current_border == new_detected_border {
            self.inconsistent_cnt = 0;
            return false;
        }

        let mut border_changed = false;
        if new_detected_border.unknown {
            if self.consistent_cnt == self.unknown_switch_cnt {
                self.current_border = new_detected_border;
                border_changed = true;
            }
        } else if self.current_border.unknown || self.consistent_cnt == self.border_switch_cnt {
            self.current_border = new_detected_border;
            border_changed = true;
        }

        border_changed
    }

    fn process_frame(&mut self, frame: &ScreenFrame<'_>) {
        if !self.enabled {
            self.current_border = BlackBorder::default();
            return;
        }

        let mut image_border = self.detector.detect(frame, self.mode);

        if image_border.horizontal_size > 0 {
            image_border.horizontal_size += self.blur_remove_cnt;
        }
        if image_border.vertical_size > 0 {
            image_border.vertical_size += self.blur_remove_cnt;
        }

        let _ = self.update_border(image_border);
    }

    fn crop_region_for(&self, frame: &ScreenFrame<'_>) -> CropRegion {
        if self.current_border.unknown {
            return CropRegion::default();
        }

        let width = frame.width.max(1) as f32;
        let height = frame.height.max(1) as f32;

        let top = (self.current_border.horizontal_size.max(0) as f32 / height).clamp(0.0, 0.45);
        let bottom = top;
        let left = (self.current_border.vertical_size.max(0) as f32 / width).clamp(0.0, 0.45);
        let right = left;

        CropRegion {
            left,
            right,
            top,
            bottom,
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
    params: &SCREEN_PARAMS,
    factory: factory,
});
