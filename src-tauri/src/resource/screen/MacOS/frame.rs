use std::sync::{Arc, RwLock};
use screencapturekit::cv::CVPixelBufferLockFlags;
use screencapturekit::prelude::*;

use super::config::BYTES_PER_PIXEL;
use crate::resource::screen::compute_scaled_dimensions_by_max_pixels;

// ============================================================================
// Frame Buffer for Stream Output
// ============================================================================

/// Thread-safe frame buffer shared between stream handler and capturer
pub(crate) struct SharedFrameBuffer {
    /// BGRA pixel data
    pub(crate) buffer: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) stride: usize,
    /// Frame counter for detecting new frames
    pub(crate) frame_id: u64,
}

impl SharedFrameBuffer {
    pub(crate) fn new() -> Self {
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
pub(crate) struct FrameHandler {
    pub(crate) frame_buffer: Arc<RwLock<SharedFrameBuffer>>,
    pub(crate) max_pixels: u32,
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

        // Calculate target dimensions based on max pixel budget
        let (target_width, target_height) = compute_scaled_dimensions_by_max_pixels(
            source_width,
            source_height,
            self.max_pixels,
        );

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
        if target_width == source_width && target_height == source_height {
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
