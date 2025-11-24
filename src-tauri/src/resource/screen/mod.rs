use std::fmt::{Display, Formatter};

/// Represents a BGRA screen frame stored in contiguous memory.
pub struct ScreenFrame<'a> {
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub pixels: &'a [u8],
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

#[cfg(target_os = "windows")]
#[path = "Windows/mod.rs"]
pub mod windows;

#[cfg(target_os = "windows")]
pub use windows::DesktopDuplicator;

#[cfg(not(target_os = "windows"))]
pub mod windows {
    use super::{ScreenCaptureError, ScreenCapturer, ScreenFrame};

    pub struct DesktopDuplicator;

    impl DesktopDuplicator {
        pub fn new() -> Result<Self, ScreenCaptureError> {
            Err(ScreenCaptureError::Unsupported(
                "Windows desktop duplication is not available on this platform",
            ))
        }
    }

    impl ScreenCapturer for DesktopDuplicator {
        fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError> {
            Err(ScreenCaptureError::Unsupported(
                "Screen capture not implemented for this platform",
            ))
        }

        fn size(&self) -> (u32, u32) {
            (0, 0)
        }
    }
}
