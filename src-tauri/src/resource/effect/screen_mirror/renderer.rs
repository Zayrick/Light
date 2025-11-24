use crate::interface::controller::Color;
use crate::resource::screen::ScreenFrame;

#[derive(Clone, Copy, Debug, Default)]
pub struct CropRegion {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

pub fn render_frame(
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

