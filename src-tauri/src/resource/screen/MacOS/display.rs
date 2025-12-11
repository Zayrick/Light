use screencapturekit::prelude::*;
use serde::Serialize;
use crate::resource::screen::ScreenCaptureError;

#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_hdr: bool,
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
