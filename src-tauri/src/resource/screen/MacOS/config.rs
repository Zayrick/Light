use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use serde::{Deserialize, Serialize};

use super::manager::global_manager;

// ============================================================================
// Constants
// ============================================================================

pub(crate) const BYTES_PER_PIXEL: usize = 4;
pub(crate) const DEFAULT_CAPTURE_FPS: u8 = 30;

// ============================================================================
// Global Settings
// ============================================================================

/// Percentage scale factor (1-100) for the capture resolution.
pub(crate) static CAPTURE_SCALE_PERCENT: AtomicU8 = AtomicU8::new(5);
pub(crate) static CAPTURE_FPS: AtomicU8 = AtomicU8::new(DEFAULT_CAPTURE_FPS);

/// Generation counter for capture state; bump when settings change.
pub(crate) static CAPTURE_GEN: AtomicU64 = AtomicU64::new(0);

// ============================================================================
// Public Types
// ============================================================================

/// Available screen capture methods (for API compatibility with Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMethod {
    /// ScreenCaptureKit (native macOS framework)
    #[default]
    ScreenCaptureKit,
}

impl std::fmt::Display for CaptureMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureMethod::ScreenCaptureKit => write!(f, "screencapturekit"),
        }
    }
}

impl std::str::FromStr for CaptureMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "screencapturekit" | "sck" => Ok(CaptureMethod::ScreenCaptureKit),
            _ => Err(format!("Unknown capture method: {}", s)),
        }
    }
}

// ============================================================================
// Public API - Settings
// ============================================================================

pub fn set_capture_scale_percent(percent: u8) {
    let clamped = percent.clamp(1, 100);
    let previous = CAPTURE_SCALE_PERCENT.swap(clamped, Ordering::Relaxed);

    if previous != clamped {
        if let Ok(mut manager) = global_manager().lock() {
            manager.clear();
        }
        CAPTURE_GEN.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn get_capture_scale_percent() -> u8 {
    CAPTURE_SCALE_PERCENT.load(Ordering::Relaxed)
}

pub fn set_capture_fps(fps: u8) {
    CAPTURE_FPS.store(fps.clamp(1, 60), Ordering::Relaxed);
}

pub fn get_capture_fps() -> u8 {
    CAPTURE_FPS.load(Ordering::Relaxed)
}

pub fn set_hardware_acceleration(_enabled: bool) {
    // ScreenCaptureKit always uses hardware acceleration
}

pub fn get_hardware_acceleration() -> bool {
    true // ScreenCaptureKit uses GPU acceleration
}

pub fn set_capture_method(_method: CaptureMethod) {
    // Only one method available for ScreenCaptureKit
}

pub fn get_capture_method() -> CaptureMethod {
    CaptureMethod::ScreenCaptureKit
}

#[allow(dead_code)]
pub fn set_sample_ratio(_percent: u8) {}

#[allow(dead_code)]
pub fn get_sample_ratio() -> u8 {
    100
}
