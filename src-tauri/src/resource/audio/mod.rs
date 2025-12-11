pub mod manager;

#[cfg(target_os = "macos")]
pub mod screencapturekit_audio;

pub use manager::{AudioManager, AudioDevice, AudioDeviceKind};

