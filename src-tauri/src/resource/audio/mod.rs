pub mod manager;

#[cfg(target_os = "macos")]
#[path = "MacOS/mod.rs"]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::SystemAudioCapture;

pub use manager::{AudioManager, AudioDevice, AudioDeviceKind};

