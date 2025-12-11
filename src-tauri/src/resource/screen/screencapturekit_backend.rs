//! macOS-specific screen capture backend using screencapturekit-rs library.
//!
//! This module provides high-performance screen capture functionality for macOS
//! using Apple's native ScreenCaptureKit framework via screencapturekit-rs bindings.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, AtomicU8, Ordering},
    Arc, Mutex, OnceLock, RwLock,
};

use screencapturekit::cv::CVPixelBufferLockFlags;
use screencapturekit::prelude::*;
use serde::{Deserialize, Serialize};

use super::{ScreenCaptureError, ScreenCapturer, ScreenFrame};

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
static CAPTURE_GEN: AtomicU64 = AtomicU64::new(0);

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
            "screencapturekit" | "sck" | "xcap" | "dxgi" | "gdi" => {
                Ok(CaptureMethod::ScreenCaptureKit)
            }
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

// ============================================================================
// Public API - Display Enumeration
// ============================================================================

pub fn list_displays() -> Result<Vec<DisplayInfo>, ScreenCaptureError> {
    let content = SCShareableContent::get().map_err(|e| ScreenCaptureError::OsError {
        context: "SCShareableContent::get",
        code: format!("{:?}", e).len() as u32,
    })?;

    let displays = content.displays();
    let mut result = Vec::with_capacity(displays.len());

    for (index, display) in displays.iter().enumerate() {
        result.push(DisplayInfo {
            index,
            name: format!("Display {}", display.display_id()),
            width: display.width(),
            height: display.height(),
            is_hdr: false, // Could be extended to detect HDR
        });
    }

    Ok(result)
}

// ============================================================================
// Frame Buffer for Stream Output
// ============================================================================

/// Thread-safe frame buffer shared between stream handler and capturer
struct SharedFrameBuffer {
    /// BGRA pixel data
    buffer: Vec<u8>,
    width: u32,
    height: u32,
    stride: usize,
    /// Frame counter for detecting new frames
    frame_id: u64,
}

impl SharedFrameBuffer {
    fn new() -> Self {
        Self {
            buffer: Vec::new(),
            width: 0,
            height: 0,
            stride: 0,
            frame_id: 0,
        }
    }
}

// ============================================================================
// Stream Output Handler
// ============================================================================

/// Handler that receives frames from SCStream and stores them in shared buffer
struct FrameHandler {
    frame_buffer: Arc<RwLock<SharedFrameBuffer>>,
    scale_percent: u8,
}

impl SCStreamOutputTrait for FrameHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, output_type: SCStreamOutputType) {
        if !matches!(output_type, SCStreamOutputType::Screen) {
            return;
        }

        // Get pixel buffer from sample
        let Some(pixel_buffer) = sample.image_buffer() else {
            return;
        };

        // Lock pixel buffer for reading
        let Ok(guard) = pixel_buffer.lock(CVPixelBufferLockFlags::READ_ONLY) else {
            return;
        };

        let source_width = guard.width() as u32;
        let source_height = guard.height() as u32;
        let bytes_per_row = guard.bytes_per_row();
        let pixels = guard.as_slice();

        // Calculate target dimensions based on scale
        let scale = self.scale_percent.clamp(1, 100) as u32;
        let target_width = (source_width * scale / 100).max(1);
        let target_height = (source_height * scale / 100).max(1);

        // Lock frame buffer for writing
        let Ok(mut frame_buffer) = self.frame_buffer.write() else {
            return;
        };

        // Resize buffer if needed
        let target_size = (target_width as usize) * (target_height as usize) * BYTES_PER_PIXEL;
        if frame_buffer.buffer.len() != target_size {
            frame_buffer.buffer.resize(target_size, 0);
        }

        // Copy or scale pixels
        if scale == 100 {
            // Direct copy - no scaling needed
            // Handle potential stride mismatch
            let expected_stride = (source_width as usize) * BYTES_PER_PIXEL;
            if bytes_per_row == expected_stride && pixels.len() == target_size {
                frame_buffer.buffer.copy_from_slice(pixels);
            } else {
                // Row-by-row copy to handle stride
                for y in 0..source_height as usize {
                    let src_offset = y * bytes_per_row;
                    let dst_offset = y * expected_stride;
                    let row_bytes = expected_stride.min(bytes_per_row);
                    if src_offset + row_bytes <= pixels.len()
                        && dst_offset + row_bytes <= frame_buffer.buffer.len()
                    {
                        frame_buffer.buffer[dst_offset..dst_offset + row_bytes]
                            .copy_from_slice(&pixels[src_offset..src_offset + row_bytes]);
                    }
                }
            }
        } else {
            // Fast nearest-neighbor downscaling
            let x_ratio = source_width as f32 / target_width as f32;
            let y_ratio = source_height as f32 / target_height as f32;

            for y in 0..target_height {
                let src_y = ((y as f32) * y_ratio) as usize;
                for x in 0..target_width {
                    let src_x = ((x as f32) * x_ratio) as usize;
                    let src_offset = src_y * bytes_per_row + src_x * BYTES_PER_PIXEL;
                    let dst_offset =
                        (y as usize) * (target_width as usize) * BYTES_PER_PIXEL
                            + (x as usize) * BYTES_PER_PIXEL;

                    if src_offset + BYTES_PER_PIXEL <= pixels.len()
                        && dst_offset + BYTES_PER_PIXEL <= frame_buffer.buffer.len()
                    {
                        frame_buffer.buffer[dst_offset..dst_offset + BYTES_PER_PIXEL]
                            .copy_from_slice(&pixels[src_offset..src_offset + BYTES_PER_PIXEL]);
                    }
                }
            }
        }

        frame_buffer.width = target_width;
        frame_buffer.height = target_height;
        frame_buffer.stride = (target_width as usize) * BYTES_PER_PIXEL;
        frame_buffer.frame_id += 1;
    }
}

// ============================================================================
// ScreenCaptureKit Capturer
// ============================================================================

/// Screen capturer using ScreenCaptureKit framework.
pub struct SCKCapturer {
    display_index: usize,
    stream: Option<SCStream>,
    frame_buffer: Arc<RwLock<SharedFrameBuffer>>,
    /// Local copy of frame for returning references
    local_buffer: Vec<u8>,
    local_width: u32,
    local_height: u32,
    local_stride: usize,
    last_frame_id: u64,
}

impl SCKCapturer {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        Self::with_output(0)
    }

    pub fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
        let mut capturer = Self {
            display_index: output_index,
            stream: None,
            frame_buffer: Arc::new(RwLock::new(SharedFrameBuffer::new())),
            local_buffer: Vec::new(),
            local_width: 0,
            local_height: 0,
            local_stride: 0,
            last_frame_id: 0,
        };

        capturer.start_stream()?;
        Ok(capturer)
    }

    fn start_stream(&mut self) -> Result<(), ScreenCaptureError> {
        // Stop existing stream if any
        if let Some(ref mut stream) = self.stream {
            let _ = stream.stop_capture();
        }

        // Get display
        let content = SCShareableContent::get().map_err(|e| ScreenCaptureError::OsError {
            context: "SCShareableContent::get",
            code: format!("{:?}", e).len() as u32,
        })?;

        let displays = content.displays();
        if self.display_index >= displays.len() {
            return Err(ScreenCaptureError::InvalidState(
                "Display index out of range",
            ));
        }

        let display = &displays[self.display_index];

        // Create content filter
        let filter = SCContentFilter::builder()
            .display(display)
            .exclude_windows(&[])
            .build();

        // Get FPS and create frame interval
        let fps = CAPTURE_FPS.load(Ordering::Relaxed).max(1) as i32;
        let frame_interval = CMTime::new(1, fps);

        // Configure stream with BGRA format (matches our expected format)
        let config = SCStreamConfiguration::new()
            .with_width(display.width())
            .with_height(display.height())
            .with_pixel_format(PixelFormat::BGRA)
            .with_shows_cursor(true)
            .with_minimum_frame_interval(&frame_interval);

        // Create stream
        let mut stream = SCStream::new(&filter, &config);

        // Create frame handler
        let scale_percent = CAPTURE_SCALE_PERCENT.load(Ordering::Relaxed);
        let handler = FrameHandler {
            frame_buffer: Arc::clone(&self.frame_buffer),
            scale_percent,
        };

        // Add output handler
        stream.add_output_handler(handler, SCStreamOutputType::Screen);

        // Start capture
        stream.start_capture().map_err(|e| ScreenCaptureError::OsError {
            context: "start_capture",
            code: format!("{:?}", e).len() as u32,
        })?;

        self.stream = Some(stream);
        Ok(())
    }

    pub fn set_output_index(&mut self, output_index: usize) -> Result<(), ScreenCaptureError> {
        if output_index != self.display_index {
            self.display_index = output_index;
            self.start_stream()?;
        }
        Ok(())
    }

    pub fn output_index(&self) -> usize {
        self.display_index
    }

    fn do_capture(&mut self) -> Result<(), ScreenCaptureError> {
        // Read from shared frame buffer
        {
            let frame_buffer = self.frame_buffer.read().map_err(|_| {
                ScreenCaptureError::InvalidState("Failed to lock frame buffer")
            })?;

            // Check if we have any frame data
            if !frame_buffer.buffer.is_empty() {
                // Copy to local buffer (we need to return a reference with our lifetime)
                self.local_buffer.clear();
                self.local_buffer.extend_from_slice(&frame_buffer.buffer);
                self.local_width = frame_buffer.width;
                self.local_height = frame_buffer.height;
                self.local_stride = frame_buffer.stride;
                self.last_frame_id = frame_buffer.frame_id;
                return Ok(());
            }
        }

        // Wait a bit for the first frame
        std::thread::sleep(std::time::Duration::from_millis(50));

        let frame_buffer = self.frame_buffer.read().map_err(|_| {
            ScreenCaptureError::InvalidState("Failed to lock frame buffer")
        })?;

        if frame_buffer.buffer.is_empty() {
            return Err(ScreenCaptureError::InvalidState("No frame available yet"));
        }

        // Copy to local buffer
        self.local_buffer.clear();
        self.local_buffer.extend_from_slice(&frame_buffer.buffer);
        self.local_width = frame_buffer.width;
        self.local_height = frame_buffer.height;
        self.local_stride = frame_buffer.stride;
        self.last_frame_id = frame_buffer.frame_id;

        Ok(())
    }
}

impl Drop for SCKCapturer {
    fn drop(&mut self) {
        if let Some(ref mut stream) = self.stream {
            let _ = stream.stop_capture();
        }
    }
}

impl ScreenCapturer for SCKCapturer {
    fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError> {
        self.do_capture()?;

        Ok(ScreenFrame {
            width: self.local_width,
            height: self.local_height,
            stride: self.local_stride,
            pixels: &self.local_buffer,
            dirty_regions: &[],
        })
    }

    fn size(&self) -> (u32, u32) {
        (self.local_width, self.local_height)
    }
}

// ============================================================================
// Unified Capturer Wrapper (API compatibility with Windows)
// ============================================================================

/// Unified screen capturer wrapper for ScreenCaptureKit backend.
pub struct DesktopDuplicator {
    capturer: SCKCapturer,
}

impl DesktopDuplicator {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        Ok(Self {
            capturer: SCKCapturer::new()?,
        })
    }

    pub fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
        Ok(Self {
            capturer: SCKCapturer::with_output(output_index)?,
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
    outputs: HashMap<usize, ManagedOutput>,
}

struct ManagedOutput {
    duplicator: DesktopDuplicator,
    ref_count: usize,
}

impl ScreenCaptureManager {
    fn new() -> Self {
        Self {
            outputs: HashMap::new(),
        }
    }

    fn acquire(&mut self, output_index: usize) -> Result<(), ScreenCaptureError> {
        if let Some(entry) = self.outputs.get_mut(&output_index) {
            entry.ref_count += 1;
            return Ok(());
        }

        let duplicator = DesktopDuplicator::with_output(output_index)?;
        self.outputs.insert(
            output_index,
            ManagedOutput {
                duplicator,
                ref_count: 1,
            },
        );
        Ok(())
    }

    fn release(&mut self, output_index: usize) {
        if let Some(entry) = self.outputs.get_mut(&output_index) {
            if entry.ref_count > 1 {
                entry.ref_count -= 1;
                return;
            }
        }
        self.outputs.remove(&output_index);
    }

    fn capture_with<F>(&mut self, output_index: usize, f: F) -> Result<bool, ScreenCaptureError>
    where
        F: FnOnce(&ScreenFrame<'_>),
    {
        let Some(entry) = self.outputs.get_mut(&output_index) else {
            return Ok(false);
        };

        match entry.duplicator.capture() {
            Ok(frame) => {
                f(&frame);
                Ok(true)
            }
            Err(err) => {
                if matches!(err, ScreenCaptureError::InvalidState(_)) {
                    self.outputs.remove(&output_index);
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
    generation: u64,
}

impl ScreenSubscription {
    pub fn new(display_index: usize) -> Result<Self, ScreenCaptureError> {
        let manager = global_manager();
        let mut guard = manager.lock().unwrap();
        let generation = CAPTURE_GEN.load(Ordering::Relaxed);
        guard.acquire(display_index)?;
        Ok(Self {
            display_index,
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
        if current_generation != self.generation {
            guard.acquire(self.display_index)?;
            self.generation = current_generation;
        }

        guard.capture_with(self.display_index, f)
    }
}

impl Drop for ScreenSubscription {
    fn drop(&mut self) {
        let manager = global_manager();
        if let Ok(mut guard) = manager.lock() {
            guard.release(self.display_index);
        }
    }
}
