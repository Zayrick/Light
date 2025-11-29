//! AudioStar Effect
//!
//! A star-shaped audio visualizer that displays frequency information as a radial pattern.
//! Based on the OpenRGBEffectsPlugin AudioStar implementation.

use crate::interface::controller::Color;
use crate::interface::effect::{
    DependencyBehavior, Effect, EffectMetadata, EffectParam, EffectParamDependency,
    EffectParamKind, SelectOption, SelectOptions,
};
use crate::resource::audio::{AudioDevice, AudioManager};
use inventory;
use serde_json::Value;
use spectrum_analyzer::scaling::divide_by_N_sqrt;
use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
use std::time::Duration;

const FFT_SIZE: usize = 1024;

/// Number of filtered FFT bins we'll work with (matches C++ 256 bins).
const FFT_BINS: usize = 256;

/// Target FPS for decay calculation.
const TARGET_FPS: f32 = 60.0;

pub struct AudioStarEffect {
    // Layout dimensions.
    width: usize,
    height: usize,

    // Animation state.
    time: f64,
    speed: f32,

    // Audio settings.
    audio_device_index: Option<usize>,
    avg_size: usize,

    // AGC (Auto Gain Control) settings - matches C++ AudioSettingsStruct.
    amplitude: f32,          // Gain multiplier (default 100)
    decay: f32,              // Decay rate percentage (default 80)
    filter_constant: f32,    // Low-pass filter constant (default 1.0)

    // Edge beat settings.
    edge_beat_enabled: bool,
    edge_beat_hue: u16,
    edge_beat_saturation: u8,
    edge_beat_sensitivity: f32,

    // FFT processing buffers.
    fft_buffer: Vec<f32>,     // Raw FFT magnitude (with peak-hold and decay)
    fft_nrml: Vec<f32>,       // Normalization array (frequency compensation)
    fft_filtered: Vec<f32>,   // Final filtered FFT output

    // Audio sample buffer.
    audio_samples: Vec<f32>,
}

impl AudioStarEffect {
    pub fn new() -> Self {
        // Default AGC settings matching C++ AudioSettingsStruct.
        // nrml_ofst = 0.04, nrml_scl = 0.5
        // Initialize normalization array (frequency compensation).
        // Higher frequencies get more gain to compensate for typical audio spectrum roll-off.
        let fft_nrml: Vec<f32> = (0..FFT_BINS)
            .map(|i| 0.04 + (0.5 * (i as f32 / FFT_BINS as f32)))
            .collect();

        Self {
            width: 0,
            height: 0,
            time: 0.0,
            speed: 50.0,
            audio_device_index: None,
            avg_size: 8, // C++ default is 8
            amplitude: 100.0,
            decay: 80.0,
            filter_constant: 1.0,
            edge_beat_enabled: false,
            edge_beat_hue: 0,
            edge_beat_saturation: 0,
            edge_beat_sensitivity: 100.0,
            fft_buffer: vec![0.0; FFT_BINS],
            fft_nrml,
            fft_filtered: vec![0.0; FFT_BINS],
            audio_samples: vec![0.0; FFT_SIZE],
        }
    }

    /// Process audio samples and update FFT data.
    /// Matches the C++ AudioSignalProcessor::Process() implementation.
    fn process_audio(&mut self) {
        let manager = AudioManager::get();

        // Read raw audio samples.
        manager.read_samples(&mut self.audio_samples);

        // Apply amplitude gain (AGC) - matches C++ fft_tmp[i] *= settings->amplitude.
        let amplified_samples: Vec<f32> = self.audio_samples.iter()
            .map(|&s| s * self.amplitude)
            .collect();

        // Apply decay to previous FFT values.
        // C++: data.fft[i] = data.fft[i] * ((float(settings->decay) / 100.0f / (60 / FPS)));
        let decay_factor = (self.decay / 100.0) / (60.0 / TARGET_FPS);
        for i in 0..FFT_BINS {
            self.fft_buffer[i] *= decay_factor;
        }

        // Apply Hann window (C++ window_mode == 1).
        let windowed = hann_window(&amplified_samples);

        // Compute FFT.
        if let Ok(spectrum) = samples_fft_to_spectrum(
            &windowed,
            manager.sample_rate().unwrap_or(44100),
            FrequencyLimit::Range(20.0, 20000.0),
            Some(&divide_by_N_sqrt),
        ) {
            // Map spectrum to our FFT bins.
            let freq_data: Vec<f32> = spectrum.data().iter().map(|(_, v)| v.val()).collect();

            // Downsample to FFT_BINS.
            let step = freq_data.len().max(1) as f32 / FFT_BINS as f32;
            for i in 0..FFT_BINS {
                let idx = (i as f32 * step) as usize;
                let raw_mag = freq_data.get(idx).copied().unwrap_or(0.0);

                // Apply normalization (frequency compensation).
                // C++: apply_window(fft_tmp, data.fft_nrml, 256);
                let normalized_mag = raw_mag * self.fft_nrml[i];

                // Apply logarithmic filter to minimize noise from very low amplitude frequencies.
                // C++: fftmag = (0.5f * log10(1.1f * fftmag)) + (0.9f * fftmag);
                let fftmag = if normalized_mag > 0.0 {
                    (0.5 * (1.1 * normalized_mag).log10()) + (0.9 * normalized_mag)
                } else {
                    0.0
                };

                // Clamp to [0, 1] range.
                // C++: if (fftmag > 1.0f) fftmag = 1.0f;
                let fftmag = fftmag.clamp(0.0, 1.0);

                // Peak-hold behavior: only update if new value is greater.
                // C++: if (fftmag > data.fft[i*2]) data.fft[i*2] = fftmag;
                if fftmag > self.fft_buffer[i] {
                    self.fft_buffer[i] = fftmag;
                }
            }
        }

        // Apply averaging over avg_size (C++ avg_mode == 0, binning mode).
        self.apply_binning_average();

        // Apply low-pass filter to get final filtered FFT.
        // C++: data.fft_fltr[i] = equalizer[i/16] * (data.fft_fltr[i] + (filter_constant * (data.fft[i] - data.fft_fltr[i])));
        for i in 0..FFT_BINS {
            self.fft_filtered[i] = self.fft_filtered[i] + 
                (self.filter_constant * (self.fft_buffer[i] - self.fft_filtered[i]));
        }
    }

    /// Apply binning average (C++ avg_mode == 0).
    fn apply_binning_average(&mut self) {
        if self.avg_size <= 1 {
            return;
        }

        // Average start bins.
        let mut sum1: f32 = 0.0;
        let mut sum2: f32 = 0.0;
        for k in 0..self.avg_size.min(FFT_BINS) {
            sum1 += self.fft_buffer[k];
            sum2 += self.fft_buffer[FFT_BINS - 1 - k];
        }
        let avg1 = sum1 / self.avg_size as f32;
        let avg2 = sum2 / self.avg_size as f32;
        for k in 0..self.avg_size.min(FFT_BINS) {
            self.fft_buffer[k] = avg1;
            self.fft_buffer[FFT_BINS - 1 - k] = avg2;
        }

        // Average middle bins.
        let mut i = 0;
        while i < FFT_BINS.saturating_sub(self.avg_size) {
            let mut sum: f32 = 0.0;
            for j in 0..self.avg_size {
                if i + j < FFT_BINS {
                    sum += self.fft_buffer[i + j];
                }
            }
            let avg = sum / self.avg_size as f32;
            for j in 0..self.avg_size {
                if i + j < FFT_BINS {
                    self.fft_buffer[i + j] = avg;
                }
            }
            i += self.avg_size;
        }
    }

    /// Calculate total amplitude from FFT bins.
    fn calculate_amplitude(&self) -> f32 {
        let mut amp = 0.0;
        for i in (0..FFT_BINS).step_by(self.avg_size) {
            amp += self.fft_filtered[i];
        }
        amp
    }

    /// Get color for a position in the star pattern.
    fn get_color(&self, x: f32, y: f32, w: f32, h: f32, amp: f32) -> Color {
        let cx = w * 0.5;
        let cy = h * 0.5;

        // Calculate angle from center.
        let angle = (x - cx).atan2(y - cy).abs();
        let pi = std::f32::consts::PI;

        // Map angle to FFT bin.
        let bin_index = ((FFT_BINS as f32 * (angle / (pi * 2.0))) as usize).min(FFT_BINS - 1);
        let freq_amp = self.fft_filtered[bin_index];

        // Calculate hue based on angle and time.
        let hue = ((angle / pi * 360.0) + self.time as f32) % 360.0;

        // Calculate value (brightness) based on frequency amplitude.
        let value = (freq_amp.powf(1.0 / (amp + 1.0)) * 255.0).min(255.0);

        let (r, g, b) = hsv_to_rgb(hue, 1.0, value / 255.0);

        // Apply edge beat effect if enabled.
        if self.edge_beat_enabled {
            let is_edge = x <= 0.0 || x >= w || y <= 0.0 || y >= h;

            if is_edge {
                // Use low frequency bins for bass beat detection.
                let bass_amp = self.fft_filtered[0] + self.fft_filtered.get(8).copied().unwrap_or(0.0);
                let edge_value = (0.01 * self.edge_beat_sensitivity * bass_amp).min(1.0);

                let (er, eg, eb) = hsv_to_rgb(
                    self.edge_beat_hue as f32,
                    self.edge_beat_saturation as f32 / 255.0,
                    edge_value,
                );

                // Screen blend mode.
                return Color {
                    r: screen_blend(r, er),
                    g: screen_blend(g, eg),
                    b: screen_blend(b, eb),
                };
            }
        }

        Color { r, g, b }
    }
}

impl Effect for AudioStarEffect {
    fn id(&self) -> String {
        "audio_star".to_string()
    }

    fn name(&self) -> String {
        "Audio Star".to_string()
    }

    fn tick(&mut self, _elapsed: Duration, buffer: &mut [Color]) {
        if buffer.is_empty() {
            return;
        }

        // Start capture if device is selected but not capturing.
        if let Some(device_index) = self.audio_device_index {
            let manager = AudioManager::get();
            if !manager.is_capturing() {
                if let Err(e) = manager.start_capture(device_index) {
                    eprintln!("[audio_star] Failed to start audio capture: {}", e);
                }
            }
        }

        // Process audio and update FFT.
        self.process_audio();

        let amp = self.calculate_amplitude();

        let width = if self.width == 0 {
            buffer.len()
        } else {
            self.width
        };
        let height = if self.height == 0 { 1 } else { self.height };

        let w = (width.saturating_sub(1)) as f32;
        let h = (height.saturating_sub(1)) as f32;

        let mut i = 0;
        for y in 0..height {
            for x in 0..width {
                if i >= buffer.len() {
                    break;
                }

                let color = self.get_color(x as f32, y as f32, w, h, amp);
                buffer[i] = color;
                i += 1;
            }
        }

        // Update animation time.
        self.time += self.speed as f64 / 60.0;
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    fn update_params(&mut self, params: Value) {
        if let Some(speed) = params.get("speed").and_then(|v| v.as_f64()) {
            self.speed = speed as f32;
        }

        if let Some(device_index) = params.get("audioDevice").and_then(|v| v.as_f64()) {
            let new_index = device_index as usize;
            let needs_restart = self.audio_device_index != Some(new_index);

            self.audio_device_index = Some(new_index);

            if needs_restart {
                let manager = AudioManager::get();
                manager.stop_capture();
                if let Err(e) = manager.start_capture(new_index) {
                    eprintln!("[audio_star] Failed to start audio capture: {}", e);
                }
            }
        }

        if let Some(avg_size) = params.get("avgSize").and_then(|v| v.as_f64()) {
            self.avg_size = (avg_size as usize).max(1);
        }

        // Edge beat parameters.
        if let Some(enabled) = params.get("edgeBeat").and_then(|v| v.as_bool()) {
            self.edge_beat_enabled = enabled;
        }

        if let Some(hue) = params.get("edgeBeatHue").and_then(|v| v.as_f64()) {
            self.edge_beat_hue = (hue as u16) % 360;
        }

        if let Some(sat) = params.get("edgeBeatSaturation").and_then(|v| v.as_f64()) {
            self.edge_beat_saturation = (sat as u8).min(255);
        }

        if let Some(sens) = params.get("edgeBeatSensitivity").and_then(|v| v.as_f64()) {
            self.edge_beat_sensitivity = sens as f32;
        }
    }
}

impl Drop for AudioStarEffect {
    fn drop(&mut self) {
        // Stop audio capture when effect is destroyed.
        AudioManager::get().stop_capture();
    }
}

/// Convert HSV to RGB.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

/// Screen blend mode for colors.
fn screen_blend(a: u8, b: u8) -> u8 {
    let af = a as f32 / 255.0;
    let bf = b as f32 / 255.0;
    ((1.0 - (1.0 - af) * (1.0 - bf)) * 255.0) as u8
}

/// Dynamic loader for audio device options.
fn load_audio_devices() -> Result<Vec<SelectOption>, String> {
    let devices: Vec<AudioDevice> = AudioManager::get().list_devices();

    if devices.is_empty() {
        return Ok(vec![SelectOption {
            label: "No devices found".to_string(),
            value: -1.0,
        }]);
    }

    Ok(devices
        .into_iter()
        .map(|d| SelectOption {
            label: d.name,
            value: d.index as f64,
        })
        .collect())
}

/// Effect parameters definition.
const AUDIO_STAR_PARAMS: [EffectParam; 8] = [
    EffectParam {
        key: "audioDevice",
        label: "音频设备",
        kind: EffectParamKind::Select {
            default: 0.0,
            options: SelectOptions::Dynamic(load_audio_devices),
        },
        dependency: None,
    },
    EffectParam {
        key: "speed",
        label: "速度",
        kind: EffectParamKind::Slider {
            min: 1.0,
            max: 100.0,
            step: 1.0,
            default: 50.0,
        },
        dependency: None,
    },
    EffectParam {
        key: "avgSize",
        label: "平滑度",
        kind: EffectParamKind::Slider {
            min: 1.0,
            max: 16.0,
            step: 1.0,
            default: 8.0,
        },
        dependency: None,
    },
    EffectParam {
        key: "edgeBeat",
        label: "边缘节拍",
        kind: EffectParamKind::Toggle { default: false },
        dependency: None,
    },
    EffectParam {
        key: "edgeBeatHue",
        label: "边缘色相",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 359.0,
            step: 1.0,
            default: 0.0,
        },
        dependency: Some(EffectParamDependency::Dependency {
            key: "edgeBeat",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Hide,
        }),
    },
    EffectParam {
        key: "edgeBeatSaturation",
        label: "边缘饱和度",
        kind: EffectParamKind::Slider {
            min: 0.0,
            max: 255.0,
            step: 1.0,
            default: 0.0,
        },
        dependency: Some(EffectParamDependency::Dependency {
            key: "edgeBeat",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Hide,
        }),
    },
    EffectParam {
        key: "edgeBeatSensitivity",
        label: "边缘灵敏度",
        kind: EffectParamKind::Slider {
            min: 1.0,
            max: 200.0,
            step: 1.0,
            default: 100.0,
        },
        dependency: Some(EffectParamDependency::Dependency {
            key: "edgeBeat",
            equals: Some(1.0),
            not_equals: None,
            behavior: DependencyBehavior::Hide,
        }),
    },
    // Hidden device kind selector for potential future use.
    EffectParam {
        key: "_deviceKind",
        label: "Device Kind",
        kind: EffectParamKind::Select {
            default: 0.0,
            options: SelectOptions::Static(&[]),
        },
        dependency: Some(EffectParamDependency::Always(DependencyBehavior::Hide)),
    },
];

fn factory() -> Box<dyn Effect> {
    Box::new(AudioStarEffect::new())
}

inventory::submit!(EffectMetadata {
    id: "audio_star",
    name: "Audio Star",
    description: Some("Star-shaped audio visualizer with frequency-based colors"),
    group: Some("Audio"),
    icon: Some("AudioLines"),
    params: &AUDIO_STAR_PARAMS,
    factory: factory,
});

