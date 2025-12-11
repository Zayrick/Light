use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::Ordering;

use crate::resource::screen::{ScreenCaptureError, ScreenCapturer, ScreenFrame};
use super::capturer::Capturer;
use super::config::{CAPTURE_GEN, CaptureMethod};

// ============================================================================
// Unified Capturer Wrapper (API compatibility with Windows)
// ============================================================================

/// Unified screen capturer wrapper for ScreenCaptureKit backend.
pub struct DesktopDuplicator {
    capturer: Capturer,
}

impl DesktopDuplicator {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        Ok(Self {
            capturer: Capturer::new()?,
        })
    }

    pub fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
        Ok(Self {
            capturer: Capturer::with_output(output_index)?,
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

pub(crate) struct ScreenCaptureManager {
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

    pub(crate) fn acquire(&mut self, output_index: usize) -> Result<(), ScreenCaptureError> {
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

    pub(crate) fn release(&mut self, output_index: usize) {
        if let Some(entry) = self.outputs.get_mut(&output_index) {
            if entry.ref_count > 1 {
                entry.ref_count -= 1;
                return;
            }
        }
        self.outputs.remove(&output_index);
    }

    pub(crate) fn capture_with<F>(&mut self, output_index: usize, f: F) -> Result<bool, ScreenCaptureError>
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

    pub(crate) fn clear(&mut self) {
        self.outputs.clear();
    }
}

static SCREEN_CAPTURE_MANAGER: OnceLock<Mutex<ScreenCaptureManager>> = OnceLock::new();

pub(crate) fn global_manager() -> &'static Mutex<ScreenCaptureManager> {
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
