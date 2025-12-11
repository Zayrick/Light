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
    get_capture_fps, get_capture_method, get_capture_scale_percent,
    get_hardware_acceleration, get_sample_ratio, list_displays,
    set_capture_fps, set_capture_method, set_capture_scale_percent,
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
    get_capture_fps, get_capture_method, get_capture_scale_percent,
    get_hardware_acceleration, get_sample_ratio, list_displays,
    set_capture_fps, set_capture_method, set_capture_scale_percent,
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
    get_capture_fps, get_capture_method, get_capture_scale_percent,
    get_hardware_acceleration, get_sample_ratio, list_displays,
    set_capture_fps, set_capture_method, set_capture_scale_percent,
    set_hardware_acceleration, set_sample_ratio,
};
