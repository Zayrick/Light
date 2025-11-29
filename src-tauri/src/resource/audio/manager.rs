//! Audio capture manager using cpal.
//!
//! Provides audio capture from both input devices (microphones) and output devices
//! (system audio loopback on Windows WASAPI).

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, Stream, StreamConfig};
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex, RwLock};

/// Global audio manager singleton.
static AUDIO_MANAGER: Lazy<AudioManager> = Lazy::new(AudioManager::new);

/// Kind of audio device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioDeviceKind {
    Input,
    Output,
}

/// Audio device information for UI selection.
#[derive(Clone, Debug)]
pub struct AudioDevice {
    pub index: usize,
    pub name: String,
    pub kind: AudioDeviceKind,
}

/// Ring buffer for audio samples with thread-safe access.
struct AudioRingBuffer {
    buffer: Vec<f32>,
    write_pos: usize,
    capacity: usize,
}

impl AudioRingBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            write_pos: 0,
            capacity,
        }
    }

    fn write(&mut self, samples: &[f32]) {
        for &sample in samples {
            self.buffer[self.write_pos] = sample;
            self.write_pos = (self.write_pos + 1) % self.capacity;
        }
    }

    /// Read the most recent `count` samples into the destination buffer.
    fn read_recent(&self, dest: &mut [f32]) {
        let count = dest.len().min(self.capacity);
        let start = if self.write_pos >= count {
            self.write_pos - count
        } else {
            self.capacity - (count - self.write_pos)
        };

        for (i, sample) in dest.iter_mut().enumerate().take(count) {
            *sample = self.buffer[(start + i) % self.capacity];
        }
    }
}

/// Active audio capture state.
struct CaptureState {
    _stream: Stream,
    buffer: Arc<Mutex<AudioRingBuffer>>,
    sample_rate: u32,
}

/// The main audio manager responsible for device enumeration and capture.
pub struct AudioManager {
    _host: Host,
    input_devices: Vec<Device>,
    output_devices: Vec<Device>,
    active_capture: RwLock<Option<CaptureState>>,
}

// SAFETY: cpal::Host and cpal::Device are Send (they manage internal handles).
// The active stream is guarded by RwLock.
unsafe impl Send for AudioManager {}
unsafe impl Sync for AudioManager {}

impl AudioManager {
    fn new() -> Self {
        let host = cpal::default_host();

        let input_devices: Vec<Device> = host
            .input_devices()
            .map(|iter| iter.collect())
            .unwrap_or_default();

        let output_devices: Vec<Device> = host
            .output_devices()
            .map(|iter| iter.collect())
            .unwrap_or_default();

        Self {
            _host: host,
            input_devices,
            output_devices,
            active_capture: RwLock::new(None),
        }
    }

    /// Get the global audio manager instance.
    pub fn get() -> &'static AudioManager {
        &AUDIO_MANAGER
    }

    /// Enumerate all available audio devices.
    pub fn list_devices(&self) -> Vec<AudioDevice> {
        let mut devices = Vec::new();

        for (i, device) in self.input_devices.iter().enumerate() {
            let name = device
                .name()
                .unwrap_or_else(|_| format!("Input Device {}", i));
            devices.push(AudioDevice {
                index: i,
                name,
                kind: AudioDeviceKind::Input,
            });
        }

        for (i, device) in self.output_devices.iter().enumerate() {
            let name = device
                .name()
                .unwrap_or_else(|_| format!("Output Device {}", i));
            devices.push(AudioDevice {
                // Output devices are indexed after input devices in the combined list
                index: self.input_devices.len() + i,
                name,
                kind: AudioDeviceKind::Output,
            });
        }

        devices
    }

    /// Get a device by combined index.
    fn device_by_index(&self, index: usize) -> Option<(&Device, AudioDeviceKind)> {
        if index < self.input_devices.len() {
            Some((&self.input_devices[index], AudioDeviceKind::Input))
        } else {
            let output_index = index - self.input_devices.len();
            self.output_devices
                .get(output_index)
                .map(|d| (d, AudioDeviceKind::Output))
        }
    }

    /// Start capturing audio from the specified device.
    pub fn start_capture(&self, device_index: usize) -> Result<(), String> {
        // Stop any existing capture first.
        self.stop_capture();

        let (device, kind) = self
            .device_by_index(device_index)
            .ok_or_else(|| format!("Invalid audio device index: {}", device_index))?;

        let config = match kind {
            AudioDeviceKind::Input => device
                .default_input_config()
                .map_err(|e| format!("No default input config: {}", e))?,
            AudioDeviceKind::Output => device
                .default_output_config()
                .map_err(|e| format!("No default output config: {}", e))?,
        };

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let sample_format = config.sample_format();

        // Allocate buffer for ~100ms of audio at the given sample rate (mono).
        let buffer_size = (sample_rate as usize / 10).max(4096);
        let buffer = Arc::new(Mutex::new(AudioRingBuffer::new(buffer_size)));
        let buffer_clone = Arc::clone(&buffer);

        let stream_config: StreamConfig = config.into();

        let err_fn = |err| {
            eprintln!("[audio] Stream error: {}", err);
        };

        // Build the input stream (for both input and output loopback devices).
        let stream = match sample_format {
            SampleFormat::F32 => {
                let callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Convert to mono by averaging channels.
                    let mono: Vec<f32> = data
                        .chunks(channels)
                        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                        .collect();

                    if let Ok(mut buf) = buffer_clone.lock() {
                        buf.write(&mono);
                    }
                };

                if kind == AudioDeviceKind::Input {
                    device
                        .build_input_stream(&stream_config, callback, err_fn, None)
                        .map_err(|e| format!("Failed to build input stream: {}", e))?
                } else {
                    // For output devices on Windows WASAPI, we need loopback capture.
                    #[cfg(target_os = "windows")]
                    {
                        device
                            .build_input_stream(&stream_config, callback, err_fn, None)
                            .map_err(|e| format!("Failed to build loopback stream: {}", e))?
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        return Err("Output loopback not supported on this platform".to_string());
                    }
                }
            }
            SampleFormat::I16 => {
                let callback = move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let mono: Vec<f32> = data
                        .chunks(channels)
                        .map(|frame| {
                            frame.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32
                        })
                        .collect();

                    if let Ok(mut buf) = buffer_clone.lock() {
                        buf.write(&mono);
                    }
                };

                if kind == AudioDeviceKind::Input {
                    device
                        .build_input_stream(&stream_config, callback, err_fn, None)
                        .map_err(|e| format!("Failed to build input stream: {}", e))?
                } else {
                    #[cfg(target_os = "windows")]
                    {
                        device
                            .build_input_stream(&stream_config, callback, err_fn, None)
                            .map_err(|e| format!("Failed to build loopback stream: {}", e))?
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        return Err("Output loopback not supported on this platform".to_string());
                    }
                }
            }
            SampleFormat::U16 => {
                let callback = move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let mono: Vec<f32> = data
                        .chunks(channels)
                        .map(|frame| {
                            frame
                                .iter()
                                .map(|&s| (s as f32 - 32768.0) / 32768.0)
                                .sum::<f32>()
                                / channels as f32
                        })
                        .collect();

                    if let Ok(mut buf) = buffer_clone.lock() {
                        buf.write(&mono);
                    }
                };

                if kind == AudioDeviceKind::Input {
                    device
                        .build_input_stream(&stream_config, callback, err_fn, None)
                        .map_err(|e| format!("Failed to build input stream: {}", e))?
                } else {
                    #[cfg(target_os = "windows")]
                    {
                        device
                            .build_input_stream(&stream_config, callback, err_fn, None)
                            .map_err(|e| format!("Failed to build loopback stream: {}", e))?
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        return Err("Output loopback not supported on this platform".to_string());
                    }
                }
            }
            _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
        };

        stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;

        let capture_state = CaptureState {
            _stream: stream,
            buffer,
            sample_rate,
        };

        if let Ok(mut guard) = self.active_capture.write() {
            *guard = Some(capture_state);
        }

        Ok(())
    }

    /// Stop the current audio capture.
    pub fn stop_capture(&self) {
        if let Ok(mut guard) = self.active_capture.write() {
            *guard = None;
        }
    }

    /// Get the current sample rate of the active capture.
    pub fn sample_rate(&self) -> Option<u32> {
        self.active_capture
            .read()
            .ok()
            .and_then(|guard| guard.as_ref().map(|state| state.sample_rate))
    }

    /// Read the most recent audio samples.
    /// Returns the number of samples actually read.
    pub fn read_samples(&self, dest: &mut [f32]) -> usize {
        if let Ok(guard) = self.active_capture.read() {
            if let Some(state) = guard.as_ref() {
                if let Ok(buf) = state.buffer.lock() {
                    buf.read_recent(dest);
                    return dest.len();
                }
            }
        }
        // Fill with zeros if no capture is active.
        dest.fill(0.0);
        0
    }

    /// Check if capture is currently active.
    pub fn is_capturing(&self) -> bool {
        self.active_capture
            .read()
            .ok()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }
}

/// Get a list of audio devices for the frontend.
pub fn list_audio_devices() -> Vec<AudioDevice> {
    AudioManager::get().list_devices()
}

