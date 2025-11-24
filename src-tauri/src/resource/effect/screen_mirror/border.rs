use crate::resource::screen::ScreenFrame;
use super::renderer::CropRegion;

#[derive(Clone, Copy, Debug, Default)]
pub struct BlackBorder {
    pub unknown: bool,
    pub horizontal_size: i32,
    pub vertical_size: i32,
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
pub enum BlackBorderMode {
    Default,
    Classic,
    Osd,
    Letterbox,
}

impl BlackBorderMode {
    pub fn from_value(value: i32) -> Self {
        match value {
            1 => BlackBorderMode::Classic,
            2 => BlackBorderMode::Osd,
            3 => BlackBorderMode::Letterbox,
            _ => BlackBorderMode::Default,
        }
    }
}

pub struct BlackBorderDetector {
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

pub struct BlackBorderProcessor {
    pub enabled: bool,
    pub unknown_switch_cnt: u32,
    pub border_switch_cnt: u32,
    pub max_inconsistent_cnt: u32,
    pub blur_remove_cnt: i32,
    pub mode: BlackBorderMode,
    detector: BlackBorderDetector,
    current_border: BlackBorder,
    previous_detected_border: BlackBorder,
    consistent_cnt: u32,
    inconsistent_cnt: u32,
}

impl BlackBorderProcessor {
    pub fn new() -> Self {
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

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.reset_state();
        }
    }

    pub fn reset_state(&mut self) {
        self.current_border = BlackBorder::default();
        self.previous_detected_border = BlackBorder::default();
        self.consistent_cnt = 0;
        self.inconsistent_cnt = self.max_inconsistent_cnt;
    }

    pub fn set_threshold_percent(&mut self, threshold_percent: f32) {
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

    pub fn process_frame(&mut self, frame: &ScreenFrame<'_>) {
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

    pub fn crop_region_for(&self, frame: &ScreenFrame<'_>) -> CropRegion {
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

