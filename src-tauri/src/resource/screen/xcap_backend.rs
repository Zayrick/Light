//! Cross-platform screen capture backend using xcap library.
//!
//! This module provides screen capture functionality for macOS and Linux
//! using the xcap library. Windows uses its native DXGI/GDI implementation.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering},
    Mutex, OnceLock,
};

use serde::{Deserialize, Serialize};
use xcap::Monitor;

use super::{
    compute_scaled_dimensions_by_max_pixels, normalize_capture_max_pixels,
    DEFAULT_CAPTURE_MAX_PIXELS, ScreenCaptureError, ScreenCapturer, ScreenFrame,
};

// ============================================================================
// Constants
// ============================================================================

pub(crate) const BYTES_PER_PIXEL: usize = 4;
pub(crate) const DEFAULT_CAPTURE_FPS: u8 = 30;

// ============================================================================
// Global Settings
// ============================================================================

/// Max pixel budget for capture resolution. 0 means "no limit".
pub(crate) static CAPTURE_MAX_PIXELS: AtomicU32 = AtomicU32::new(DEFAULT_CAPTURE_MAX_PIXELS);
pub(crate) static CAPTURE_FPS: AtomicU8 = AtomicU8::new(DEFAULT_CAPTURE_FPS);

/// Generation counter for capture state; bump when settings change.
static CAPTURE_GEN: AtomicU64 = AtomicU64::new(0);

// ============================================================================
// Public Types
// ============================================================================

/// Available screen capture methods (for API compatibility with Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMethod {
    /// Default xcap method
    #[default]
    Xcap,
}

impl std::fmt::Display for CaptureMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureMethod::Xcap => write!(f, "xcap"),
        }
    }
}

impl std::str::FromStr for CaptureMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "xcap" | "dxgi" | "gdi" => Ok(CaptureMethod::Xcap),
            _ => Err(format!("Unknown capture method: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_hdr: bool,
}

// ============================================================================
// Public API - Settings
// ============================================================================

pub fn set_capture_max_pixels(max_pixels: u32) {
    let normalized = normalize_capture_max_pixels(max_pixels);
    let previous = CAPTURE_MAX_PIXELS.swap(normalized, Ordering::Relaxed);

    if previous != normalized {
        if let Ok(mut manager) = global_manager().lock() {
            manager.clear();
        }
        CAPTURE_GEN.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn get_capture_max_pixels() -> u32 {
    CAPTURE_MAX_PIXELS.load(Ordering::Relaxed)
}

pub fn set_capture_fps(fps: u8) {
    CAPTURE_FPS.store(fps.clamp(1, 60), Ordering::Relaxed);
}

pub fn get_capture_fps() -> u8 {
    CAPTURE_FPS.load(Ordering::Relaxed)
}

pub fn set_hardware_acceleration(_enabled: bool) {
    // Not applicable for xcap backend
}

pub fn get_hardware_acceleration() -> bool {
    false
}

pub fn set_capture_method(_method: CaptureMethod) {
    // Only one method available for xcap
}

pub fn get_capture_method() -> CaptureMethod {
    CaptureMethod::Xcap
}

#[allow(dead_code)]
pub fn set_sample_ratio(_percent: u8) {}

#[allow(dead_code)]
pub fn get_sample_ratio() -> u8 {
    100
}

// ============================================================================
// Public API - Display Enumeration
// ============================================================================

pub fn list_displays() -> Result<Vec<DisplayInfo>, ScreenCaptureError> {
    let monitors = Monitor::all().map_err(|e| {
        ScreenCaptureError::OsError {
            context: "Monitor::all",
            code: e.to_string().len() as u32,
        }
    })?;

    let mut displays = Vec::new();
    for (index, monitor) in monitors.iter().enumerate() {
        let name = monitor.name().unwrap_or_else(|_| format!("Display {}", index));
        let width = monitor.width().unwrap_or(0);
        let height = monitor.height().unwrap_or(0);

        displays.push(DisplayInfo {
            index,
            name,
            width,
            height,
            is_hdr: false, // xcap doesn't expose HDR info
        });
    }

    Ok(displays)
}

// ============================================================================
// XCap Capturer
// ============================================================================

/// Screen capturer using xcap library.
pub struct XcapCapturer {
    monitor_index: usize,
    // Cached frame buffer in BGRA format
    buffer: Vec<u8>,
    width: u32,
    height: u32,
    stride: usize,
}

impl XcapCapturer {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        Self::with_output(0)
    }

    pub fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
        let monitors = Monitor::all().map_err(|e| {
            ScreenCaptureError::OsError {
                context: "Monitor::all",
                code: e.to_string().len() as u32,
            }
        })?;

        if output_index >= monitors.len() {
            return Err(ScreenCaptureError::InvalidState("Monitor index out of range"));
        }

        let monitor = &monitors[output_index];
        let width = monitor.width().unwrap_or(1920);
        let height = monitor.height().unwrap_or(1080);

        Ok(Self {
            monitor_index: output_index,
            buffer: Vec::new(),
            width,
            height,
            stride: (width as usize) * BYTES_PER_PIXEL,
        })
    }

    pub fn set_output_index(&mut self, output_index: usize) -> Result<(), ScreenCaptureError> {
        let monitors = Monitor::all().map_err(|e| {
            ScreenCaptureError::OsError {
                context: "Monitor::all",
                code: e.to_string().len() as u32,
            }
        })?;

        if output_index >= monitors.len() {
            return Err(ScreenCaptureError::InvalidState("Monitor index out of range"));
        }

        let monitor = &monitors[output_index];
        self.monitor_index = output_index;
        self.width = monitor.width().unwrap_or(1920);
        self.height = monitor.height().unwrap_or(1080);
        self.stride = (self.width as usize) * BYTES_PER_PIXEL;

        Ok(())
    }

    pub fn output_index(&self) -> usize {
        self.monitor_index
    }

    fn do_capture(&mut self) -> Result<(), ScreenCaptureError> {
        let monitors = Monitor::all().map_err(|e| {
            ScreenCaptureError::OsError {
                context: "Monitor::all",
                code: e.to_string().len() as u32,
            }
        })?;

        if self.monitor_index >= monitors.len() {
            return Err(ScreenCaptureError::InvalidState("Monitor index out of range"));
        }

        let monitor = &monitors[self.monitor_index];

        // Capture the screen
        let image = monitor.capture_image().map_err(|e| {
            ScreenCaptureError::OsError {
                context: "capture_image",
                code: e.to_string().len() as u32,
            }
        })?;

        // Apply max pixel budget
        let max_pixels = CAPTURE_MAX_PIXELS.load(Ordering::Relaxed);
        let source_width = image.width();
        let source_height = image.height();

        let (target_width, target_height) = compute_scaled_dimensions_by_max_pixels(
            source_width,
            source_height,
            max_pixels,
        );

        let (scaled_image, final_width, final_height) = if target_width != source_width
            || target_height != source_height
        {

            // Use fast nearest-neighbor resize for performance
            let resized = image::imageops::resize(
                &image,
                target_width.max(1),
                target_height.max(1),
                image::imageops::FilterType::Nearest,
            );
            (resized.into_raw(), target_width, target_height)
        } else {
            (image.into_raw(), source_width, source_height)
        };

        self.width = final_width;
        self.height = final_height;
        self.stride = (self.width as usize) * BYTES_PER_PIXEL;

        // Convert RGBA to BGRA format (xcap returns RGBA, our system expects BGRA)
        self.buffer.clear();
        self.buffer.reserve(scaled_image.len());

        for chunk in scaled_image.chunks_exact(4) {
            // RGBA -> BGRA
            self.buffer.push(chunk[2]); // B
            self.buffer.push(chunk[1]); // G
            self.buffer.push(chunk[0]); // R
            self.buffer.push(chunk[3]); // A
        }

        Ok(())
    }
}

impl ScreenCapturer for XcapCapturer {
    fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError> {
        self.do_capture()?;

        Ok(ScreenFrame {
            width: self.width,
            height: self.height,
            stride: self.stride,
            pixels: &self.buffer,
            dirty_regions: &[],
        })
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

// ============================================================================
// Unified Capturer Wrapper (API compatibility with Windows)
// ============================================================================

/// Unified screen capturer wrapper for xcap backend.
pub struct DesktopDuplicator {
    capturer: XcapCapturer,
}

impl DesktopDuplicator {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        Ok(Self {
            capturer: XcapCapturer::new()?,
        })
    }

    pub fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
        Ok(Self {
            capturer: XcapCapturer::with_output(output_index)?,
        })
    }

    pub fn with_method_output(
        _method: CaptureMethod,
        output_index: usize,
    ) -> Result<Self, ScreenCaptureError> {
        Self::with_output(output_index)
    }

    pub fn set_output_index(&mut self, output_index: usize) -> Result<(), ScreenCaptureError> {
        self.capturer.set_output_index(output_index)
    }

    pub fn output_index(&self) -> usize {
        self.capturer.output_index()
    }
}

impl ScreenCapturer for DesktopDuplicator {
    fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError> {
        self.capturer.capture()
    }

    fn size(&self) -> (u32, u32) {
        self.capturer.size()
    }
}

// ============================================================================
// Screen Capture Manager
// ============================================================================

struct ScreenCaptureManager {
    outputs: HashMap<CaptureKey, ManagedOutput>,
}

struct ManagedOutput {
    duplicator: DesktopDuplicator,
    ref_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CaptureKey {
    method: CaptureMethod,
    output: usize,
}

impl ScreenCaptureManager {
    fn new() -> Self {
        Self {
            outputs: HashMap::new(),
        }
    }

    fn acquire(
        &mut self,
        method: CaptureMethod,
        output_index: usize,
    ) -> Result<(), ScreenCaptureError> {
        let key = CaptureKey {
            method,
            output: output_index,
        };

        if let Some(entry) = self.outputs.get_mut(&key) {
            entry.ref_count += 1;
            return Ok(());
        }

        let duplicator = DesktopDuplicator::with_method_output(method, output_index)?;
        self.outputs.insert(
            key,
            ManagedOutput {
                duplicator,
                ref_count: 1,
            },
        );
        Ok(())
    }

    fn release(&mut self, key: CaptureKey) {
        if let Some(entry) = self.outputs.get_mut(&key) {
            if entry.ref_count > 1 {
                entry.ref_count -= 1;
                return;
            }
        }
        self.outputs.remove(&key);
    }

    fn capture_with<F>(&mut self, key: CaptureKey, f: F) -> Result<bool, ScreenCaptureError>
    where
        F: FnOnce(&ScreenFrame<'_>),
    {
        let Some(entry) = self.outputs.get_mut(&key) else {
            return Ok(false);
        };

        match entry.duplicator.capture() {
            Ok(frame) => {
                f(&frame);
                Ok(true)
            }
            Err(err) => {
                if matches!(err, ScreenCaptureError::InvalidState(_)) {
                    self.outputs.remove(&key);
                }
                Err(err)
            }
        }
    }

    fn clear(&mut self) {
        self.outputs.clear();
    }
}

static SCREEN_CAPTURE_MANAGER: OnceLock<Mutex<ScreenCaptureManager>> = OnceLock::new();

fn global_manager() -> &'static Mutex<ScreenCaptureManager> {
    SCREEN_CAPTURE_MANAGER.get_or_init(|| Mutex::new(ScreenCaptureManager::new()))
}

// ============================================================================
// Screen Subscription
// ============================================================================

/// RAII handle for a display subscription.
#[derive(Debug)]
pub struct ScreenSubscription {
    display_index: usize,
    method: CaptureMethod,
    generation: u64,
}

impl ScreenSubscription {
    pub fn new(display_index: usize) -> Result<Self, ScreenCaptureError> {
        let manager = global_manager();
        let mut guard = manager.lock().unwrap();
        let method = get_capture_method();
        let generation = CAPTURE_GEN.load(Ordering::Relaxed);
        guard.acquire(method, display_index)?;
        Ok(Self {
            display_index,
            method,
            generation,
        })
    }

    pub fn display_index(&self) -> usize {
        self.display_index
    }

    pub fn capture_with<F>(&mut self, f: F) -> Result<bool, ScreenCaptureError>
    where
        F: FnOnce(&ScreenFrame<'_>),
    {
        let manager = global_manager();
        let mut guard = manager.lock().unwrap();

        let current_generation = CAPTURE_GEN.load(Ordering::Relaxed);
        let current_method = get_capture_method();
        if current_generation != self.generation || current_method != self.method {
            guard.acquire(current_method, self.display_index)?;
            self.generation = current_generation;
            self.method = current_method;
        }

        let key = CaptureKey {
            method: self.method,
            output: self.display_index,
        };

        guard.capture_with(key, f)
    }
}

impl Drop for ScreenSubscription {
    fn drop(&mut self) {
        let manager = global_manager();
        if let Ok(mut guard) = manager.lock() {
            let key = CaptureKey {
                method: self.method,
                output: self.display_index,
            };
            guard.release(key);
        }
    }
}
