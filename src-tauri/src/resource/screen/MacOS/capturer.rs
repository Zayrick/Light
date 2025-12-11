use std::sync::{Arc, RwLock};
use std::sync::atomic::Ordering;
use screencapturekit::prelude::*;

use crate::resource::screen::{ScreenCaptureError, ScreenCapturer, ScreenFrame};
use super::frame::{FrameHandler, SharedFrameBuffer};
use super::config::{CAPTURE_FPS, CAPTURE_SCALE_PERCENT};

// ============================================================================
// ScreenCaptureKit Capturer
// ============================================================================

/// Screen capturer using ScreenCaptureKit framework.
pub(crate) struct Capturer {
    display_index: usize,
    stream: Option<SCStream>,
    frame_buffer: Arc<RwLock<SharedFrameBuffer>>,
    /// Local copy of frame for returning references
    local_buffer: Vec<u8>,
    local_width: u32,
    local_height: u32,
    local_stride: usize,
    #[allow(dead_code)] // Useful for debugging or future extension
    last_frame_id: u64,
}

impl Capturer {
    pub(crate) fn new() -> Result<Self, ScreenCaptureError> {
        Self::with_output(0)
    }

    pub(crate) fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
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

    pub(crate) fn set_output_index(&mut self, output_index: usize) -> Result<(), ScreenCaptureError> {
        if output_index != self.display_index {
            self.display_index = output_index;
            self.start_stream()?;
        }
        Ok(())
    }

    pub(crate) fn output_index(&self) -> usize {
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

impl Drop for Capturer {
    fn drop(&mut self) {
        if let Some(ref mut stream) = self.stream {
            let _ = stream.stop_capture();
        }
    }
}

impl ScreenCapturer for Capturer {
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
