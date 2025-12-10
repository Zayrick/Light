//! Windows screen capture module with multiple backend support.
//!
//! This module provides screen capture functionality with two backends:
//! - DXGI (Desktop Duplication API): High performance, GPU accelerated, HDR support
//! - GDI (Graphics Device Interface): Better compatibility with older systems

#[path = "DXGI/mod.rs"]
pub mod dxgi;
#[path = "GDI/mod.rs"]
pub mod gdi;

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    Mutex, OnceLock, RwLock,
};

use serde::{Deserialize, Serialize};

use windows::core::Interface;
use windows::Win32::Graphics::Dxgi::{
    Common::DXGI_COLOR_SPACE_TYPE,
    CreateDXGIFactory1, IDXGIFactory1, IDXGIOutput6,
    DXGI_ERROR_NOT_FOUND, DXGI_OUTPUT_DESC,
};

use super::{ScreenCaptureError, ScreenCapturer, ScreenFrame};
use dxgi::DxgiCapturer;
use gdi::GdiCapturer;

// ============================================================================
// Constants
// ============================================================================

/// HDR color space constant (DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020 = 12)
pub(crate) const HDR_COLOR_SPACE: DXGI_COLOR_SPACE_TYPE = DXGI_COLOR_SPACE_TYPE(12);

pub(crate) const BYTES_PER_PIXEL: usize = 4;
pub(crate) const DEFAULT_TIMEOUT_MS: u32 = 16;
pub(crate) const DEFAULT_CAPTURE_FPS: u8 = 30;
pub(crate) const DEFAULT_TARGET_NITS: u32 = 200;

// ============================================================================
// Global Settings
// ============================================================================

/// Percentage scale factor (1-100) for the capture resolution.
pub(crate) static CAPTURE_SCALE_PERCENT: AtomicU8 = AtomicU8::new(5);
pub(crate) static CAPTURE_FPS: AtomicU8 = AtomicU8::new(DEFAULT_CAPTURE_FPS);
pub(crate) static HARDWARE_ACCELERATION: AtomicBool = AtomicBool::new(true);

/// Screen capture method selection
static CAPTURE_METHOD: RwLock<CaptureMethod> = RwLock::new(CaptureMethod::Dxgi);
/// Generation counter for capture state; bump when method changes to help
/// existing subscriptions re-sync without manual toggles.
static CAPTURE_GEN: AtomicU64 = AtomicU64::new(0);

// ============================================================================
// Public Types
// ============================================================================

/// Available screen capture methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptureMethod {
    /// DXGI Desktop Duplication API (default, high performance, HDR support)
    #[default]
    Dxgi,
    /// GDI (Graphics Device Interface, better compatibility)
    Gdi,
}

impl std::fmt::Display for CaptureMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptureMethod::Dxgi => write!(f, "dxgi"),
            CaptureMethod::Gdi => write!(f, "gdi"),
        }
    }
}

impl std::str::FromStr for CaptureMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dxgi" => Ok(CaptureMethod::Dxgi),
            "gdi" => Ok(CaptureMethod::Gdi),
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

pub fn set_capture_scale_percent(percent: u8) {
    CAPTURE_SCALE_PERCENT.store(percent.clamp(1, 100), Ordering::Relaxed);
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

pub fn set_hardware_acceleration(enabled: bool) {
    HARDWARE_ACCELERATION.store(enabled, Ordering::Relaxed);
}

pub fn get_hardware_acceleration() -> bool {
    HARDWARE_ACCELERATION.load(Ordering::Relaxed)
}

pub fn set_capture_method(method: CaptureMethod) {
    if let Ok(mut guard) = CAPTURE_METHOD.write() {
        *guard = method;
    }
    // Clear existing captures when method changes
    if let Ok(mut manager) = global_manager().lock() {
        manager.clear();
    }
    // Bump generation so existing subscriptions can re-sync.
    CAPTURE_GEN.fetch_add(1, Ordering::Relaxed);
}

pub fn get_capture_method() -> CaptureMethod {
    CAPTURE_METHOD.read().map(|g| *g).unwrap_or_default()
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
    unsafe {
        let factory: IDXGIFactory1 =
            CreateDXGIFactory1().map_err(|err| os_error("CreateDXGIFactory1", err))?;
        let mut displays = Vec::new();
        let mut current_index = 0usize;

        for adapter_index in 0.. {
            let adapter = match factory.EnumAdapters1(adapter_index) {
                Ok(adapter) => adapter,
                Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(err) => return Err(os_error("EnumAdapters1", err)),
            };

            for output_index in 0.. {
                let output = match adapter.EnumOutputs(output_index) {
                    Ok(output) => output,
                    Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                    Err(err) => return Err(os_error("IDXGIAdapter::EnumOutputs", err)),
                };

                let desc = output
                    .GetDesc()
                    .map_err(|err| os_error("IDXGIOutput::GetDesc", err))?;
                if !desc.AttachedToDesktop.as_bool() {
                    continue;
                }

                // Check HDR support via IDXGIOutput6
                let is_hdr = if let Ok(output6) = output.cast::<IDXGIOutput6>() {
                    if let Ok(desc1) = output6.GetDesc1() {
                        desc1.ColorSpace == HDR_COLOR_SPACE
                    } else {
                        false
                    }
                } else {
                    false
                };

                let (width, height) = output_dimensions(&desc);
                let raw_name = wide_to_string(&desc.DeviceName);
                let fallback = format!("Display {}", current_index + 1);
                let name = if raw_name.trim().is_empty() {
                    fallback
                } else {
                    raw_name
                };

                displays.push(DisplayInfo {
                    index: current_index,
                    name,
                    width,
                    height,
                    is_hdr,
                });

                current_index += 1;
            }
        }

        Ok(displays)
    }
}

// ============================================================================
// Unified Capturer Wrapper
// ============================================================================

/// Unified screen capturer that wraps either DXGI or GDI backend.
pub enum DesktopDuplicator {
    Dxgi(DxgiCapturer),
    Gdi(GdiCapturer),
}

impl DesktopDuplicator {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        Self::with_output(0)
    }

    pub fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
        let method = get_capture_method();
        Self::with_method_output(method, output_index)
    }

    pub fn with_method_output(
        method: CaptureMethod,
        output_index: usize,
    ) -> Result<Self, ScreenCaptureError> {
        match method {
            CaptureMethod::Dxgi => match DxgiCapturer::with_output(output_index) {
                Ok(capturer) => Ok(Self::Dxgi(capturer)),
                Err(err) => {
                    // Try to fall back to GDI if DXGI fails
                    eprintln!("[screen] DXGI failed, falling back to GDI: {}", err);
                    Ok(Self::Gdi(GdiCapturer::with_output(output_index)?))
                }
            },
            CaptureMethod::Gdi => Ok(Self::Gdi(GdiCapturer::with_output(output_index)?)),
        }
    }

    pub fn set_output_index(&mut self, output_index: usize) -> Result<(), ScreenCaptureError> {
        match self {
            Self::Dxgi(capturer) => capturer.set_output_index(output_index),
            Self::Gdi(capturer) => GdiCapturer::with_output(output_index).map(|c| *capturer = c),
        }
    }

    pub fn output_index(&self) -> usize {
        match self {
            Self::Dxgi(capturer) => capturer.output_index(),
            Self::Gdi(capturer) => capturer.output_index(),
        }
    }
}

impl ScreenCapturer for DesktopDuplicator {
    fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError> {
        match self {
            Self::Dxgi(capturer) => capturer.capture(),
            Self::Gdi(capturer) => capturer.capture(),
        }
    }

    fn size(&self) -> (u32, u32) {
        match self {
            Self::Dxgi(capturer) => capturer.size(),
            Self::Gdi(capturer) => capturer.size(),
        }
    }
}

// ============================================================================
// Screen Capture Manager
// ============================================================================

/// Shares one `DesktopDuplicator` per display and frees it when unused.
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

        // Refresh backend binding if generation or method changed.
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

// ============================================================================
// Helper Functions
// ============================================================================

fn output_dimensions(desc: &DXGI_OUTPUT_DESC) -> (u32, u32) {
    let width = (desc.DesktopCoordinates.right - desc.DesktopCoordinates.left).max(1) as u32;
    let height = (desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top).max(1) as u32;
    (width, height)
}

fn wide_to_string(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

fn os_error(context: &'static str, err: windows::core::Error) -> ScreenCaptureError {
    ScreenCaptureError::OsError {
        context,
        code: err.code().0 as u32,
    }
}
