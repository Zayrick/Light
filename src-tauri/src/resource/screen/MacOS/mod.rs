//! macOS-specific screen capture backend using screencapturekit-rs library.
//!
//! This module provides high-performance screen capture functionality for macOS
//! using Apple's native ScreenCaptureKit framework via screencapturekit-rs bindings.

pub mod config;
pub mod display;
pub(crate) mod frame;
pub(crate) mod capturer;
pub mod manager;

pub use config::{
    CaptureMethod,
    get_capture_fps, set_capture_fps,
    get_capture_scale_percent, set_capture_scale_percent,
    get_hardware_acceleration, set_hardware_acceleration,
    get_sample_ratio, set_sample_ratio,
    get_capture_method, set_capture_method,
};

pub use display::{DisplayInfo, list_displays};

pub use manager::{DesktopDuplicator, ScreenSubscription};
