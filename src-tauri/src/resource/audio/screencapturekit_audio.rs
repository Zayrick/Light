//! macOS system audio capture using ScreenCaptureKit.
//!
//! This module provides system audio loopback capture for macOS using the native
//! ScreenCaptureKit framework, which supports capturing system audio output.

use super::manager::AudioRingBuffer;
use screencapturekit::prelude::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
};

/// No-op screen handler (required for stream to function, but we ignore video frames)
struct NoOpScreenHandler;

impl SCStreamOutputTrait for NoOpScreenHandler {
    fn did_output_sample_buffer(&self, _sample: CMSampleBuffer, _output_type: SCStreamOutputType) {
        // Intentionally ignore screen frames - we only care about audio
    }
}

/// Stream output handler for audio capture
struct AudioHandler {
    buffer: Arc<Mutex<AudioRingBuffer>>,
    sample_rate: Arc<RwLock<u32>>,
}

impl SCStreamOutputTrait for AudioHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, output_type: SCStreamOutputType) {
        // Process audio samples (received alongside Screen type when audio is enabled)
        if !matches!(output_type, SCStreamOutputType::Audio) {
            return;
        }

        // Get audio buffer list
        let Some(audio_list) = sample.audio_buffer_list() else {
            return;
        };

        // Try to get format description for sample rate
        if let Some(format_desc) = sample.format_description() {
            if let Some(rate) = format_desc.audio_sample_rate() {
                if let Ok(mut sr) = self.sample_rate.write() {
                    *sr = rate as u32;
                }
            }
        }

        // Process each audio buffer
        for audio_buffer in audio_list.iter() {
            let data = audio_buffer.data();
            let channels = audio_buffer.number_channels as usize;

            if data.is_empty() || channels == 0 {
                continue;
            }

            // Convert bytes to f32 samples (assuming 32-bit float format)
            let float_samples: &[f32] = unsafe {
                std::slice::from_raw_parts(
                    data.as_ptr() as *const f32,
                    data.len() / std::mem::size_of::<f32>(),
                )
            };

            // Convert to mono by averaging channels
            let mono: Vec<f32> = float_samples
                .chunks(channels)
                .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                .collect();

            if let Ok(mut buf) = self.buffer.lock() {
                buf.write(&mono);
            }
        }
    }
}

/// System audio capture state using ScreenCaptureKit
pub struct SystemAudioCapture {
    stream: SCStream,
    buffer: Arc<Mutex<AudioRingBuffer>>,
    sample_rate: Arc<RwLock<u32>>,
    is_running: AtomicBool,
}

impl SystemAudioCapture {
    /// Create a new system audio capture instance.
    pub fn new() -> Result<Self, String> {
        // Get a display to create the stream (required even for audio-only capture)
        let content = SCShareableContent::get()
            .map_err(|e| format!("Failed to get shareable content: {:?}", e))?;

        let display = content
            .displays()
            .into_iter()
            .next()
            .ok_or_else(|| "No displays found".to_string())?;

        // Create content filter for the display
        let filter = SCContentFilter::builder()
            .display(&display)
            .exclude_windows(&[])
            .build();

        // Configure stream for audio + minimal video
        // Note: ScreenCaptureKit requires valid video dimensions even when we only want audio
        // Using small but valid dimensions to minimize overhead
        let config = SCStreamConfiguration::new()
            .with_width(64)
            .with_height(64)
            .with_captures_audio(true) // Enable system audio capture
            .with_excludes_current_process_audio(true) // Prevent feedback
            .with_sample_rate(48000) // 48kHz
            .with_channel_count(2); // Stereo

        // Create shared buffer (~100ms at 48kHz)
        let buffer_size = 4800;
        let buffer = Arc::new(Mutex::new(AudioRingBuffer::new(buffer_size)));
        let sample_rate = Arc::new(RwLock::new(48000u32));

        // Create handler
        let handler = AudioHandler {
            buffer: Arc::clone(&buffer),
            sample_rate: Arc::clone(&sample_rate),
        };

        // Create stream and add output handlers
        // Need to register both Screen and Audio handlers for audio to work properly
        let mut stream = SCStream::new(&filter, &config);
        // Screen handler is required for the stream to function
        stream.add_output_handler(NoOpScreenHandler, SCStreamOutputType::Screen);
        // Audio handler processes the actual audio data
        stream.add_output_handler(handler, SCStreamOutputType::Audio);

        Ok(Self {
            stream,
            buffer,
            sample_rate,
            is_running: AtomicBool::new(false),
        })
    }

    /// Start capturing system audio.
    pub fn start(&mut self) -> Result<(), String> {
        if self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.stream.start_capture().map_err(|e| {
            format!(
                "Failed to start audio capture: {:?}. \
                 Please ensure Screen Recording permission is granted in \
                 System Settings > Privacy & Security > Screen Recording.",
                e
            )
        })?;

        self.is_running.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// Stop capturing system audio.
    pub fn stop(&mut self) -> Result<(), String> {
        if !self.is_running.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.stream
            .stop_capture()
            .map_err(|e| format!("Failed to stop audio capture: {:?}", e))?;

        self.is_running.store(false, Ordering::Relaxed);
        Ok(())
    }

    /// Get the current sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
            .read()
            .map(|sr| *sr)
            .unwrap_or(48000)
    }

    /// Read the most recent audio samples into the destination buffer.
    /// Returns the number of samples actually read.
    pub fn read_samples(&self, dest: &mut [f32]) -> usize {
        if !self.is_running.load(Ordering::Relaxed) {
            dest.fill(0.0);
            return 0;
        }

        if let Ok(buf) = self.buffer.lock() {
            buf.read_recent(dest);
            dest.len()
        } else {
            dest.fill(0.0);
            0
        }
    }

    /// Check if capture is currently running.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }
}

impl Drop for SystemAudioCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

