use std::fmt::{Display, Formatter};

/// Represents a BGRA screen frame stored in contiguous memory.
pub struct ScreenFrame<'a> {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub pixels: &'a [u8],
    /// Optional dirty regions reported by the backend. Empty means "unknown/entire frame".
    pub dirty_regions: &'a [DirtyRegion],
}

/// A rectangular dirty region within a captured frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct DirtyRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Errors that can occur while capturing the screen.
#[derive(Debug)]
pub enum ScreenCaptureError {
    Unsupported(&'static str),
    OsError { context: &'static str, code: u32 },
    InvalidState(&'static str),
}

impl Display for ScreenCaptureError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenCaptureError::Unsupported(ctx) => {
                write!(f, "Screen capture unsupported: {}", ctx)
            }
            ScreenCaptureError::OsError { context, code } => {
                write!(f, "Screen capture OS error ({}): 0x{:08X}", context, code)
            }
            ScreenCaptureError::InvalidState(ctx) => {
                write!(f, "Screen capture invalid state: {}", ctx)
            }
        }
    }
}

impl std::error::Error for ScreenCaptureError {}

/// Common interface for platform specific screen capture backends.
pub trait ScreenCapturer {
    fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError>;
    fn size(&self) -> (u32, u32);
}

// ============================================================================
// Capture scaling helpers
// ============================================================================

/// Preset pixel budgets based on typical 16:9 resolutions (highest to lowest).
pub(crate) const CAPTURE_PIXEL_PRESETS: &[u32] = &[
    2_073_600, // 1080p
    921_600,   // 720p
    518_400,   // 540p
    230_400,   // 360p
    129_600,   // 270p
    57_600,    // 180p
    14_400,    // 90p
    3_600,     // 45p
    2_304,     // 36p
    576,       // 18p
];

pub(crate) const DEFAULT_CAPTURE_MAX_PIXELS: u32 = 2_304; // 36p

pub(crate) fn normalize_capture_max_pixels(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }

    let mut closest = CAPTURE_PIXEL_PRESETS[0];
    let mut closest_delta = closest.abs_diff(value);

    for &preset in &CAPTURE_PIXEL_PRESETS[1..] {
        let delta = preset.abs_diff(value);
        if delta < closest_delta || (delta == closest_delta && preset > closest) {
            closest = preset;
            closest_delta = delta;
        }
    }

    closest
}

pub(crate) fn compute_scaled_dimensions_by_max_pixels(
    width: u32,
    height: u32,
    max_pixels: u32,
) -> (u32, u32) {
    let mut scaled_width = width.max(1);
    let mut scaled_height = height.max(1);

    if max_pixels == 0 {
        return (scaled_width, scaled_height);
    }

    let max_pixels = max_pixels as u64;
    while (scaled_width as u64) * (scaled_height as u64) > max_pixels
        && (scaled_width > 1 || scaled_height > 1)
    {
        scaled_width = (scaled_width / 2).max(1);
        scaled_height = (scaled_height / 2).max(1);
    }

    (scaled_width, scaled_height)
}

// ============================================================================
// Platform-specific modules
// ============================================================================

// Windows: Use native DXGI/GDI implementation
#[cfg(target_os = "windows")]
#[path = "Windows/mod.rs"]
#[allow(clippy::module_inception)]
mod screen;

#[cfg(target_os = "windows")]
pub use screen::{
    CaptureMethod, DesktopDuplicator, DisplayInfo, ScreenSubscription,
    get_capture_fps, get_capture_method, get_capture_max_pixels,
    get_hardware_acceleration, get_sample_ratio, list_displays,
    set_capture_fps, set_capture_method, set_capture_max_pixels,
    set_hardware_acceleration, set_sample_ratio,
};

// macOS: Use ScreenCaptureKit backend (native Apple framework)
#[cfg(target_os = "macos")]
#[path = "MacOS/mod.rs"]
#[allow(clippy::module_inception)]
mod screen;

#[cfg(target_os = "macos")]
pub use screen::{
    CaptureMethod, DesktopDuplicator, DisplayInfo, ScreenSubscription,
    get_capture_fps, get_capture_method, get_capture_max_pixels,
    get_hardware_acceleration, get_sample_ratio, list_displays,
    set_capture_fps, set_capture_method, set_capture_max_pixels,
    set_hardware_acceleration, set_sample_ratio,
};

// Linux: Use xcap backend
#[cfg(target_os = "linux")]
#[path = "xcap_backend.rs"]
#[allow(clippy::module_inception)]
mod screen;

#[cfg(target_os = "linux")]
pub use screen::{
    CaptureMethod, DesktopDuplicator, DisplayInfo, ScreenSubscription,
    get_capture_fps, get_capture_method, get_capture_max_pixels,
    get_hardware_acceleration, get_sample_ratio, list_displays,
    set_capture_fps, set_capture_method, set_capture_max_pixels,
    set_hardware_acceleration, set_sample_ratio,
};
