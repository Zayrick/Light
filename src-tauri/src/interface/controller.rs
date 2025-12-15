use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// High-level device classification (inspired by OpenRGB).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceType {
    Motherboard,
    Dram,
    Gpu,
    Cooler,
    LedStrip,
    Keyboard,
    Mouse,
    MouseMat,
    Headset,
    HeadsetStand,
    Gamepad,
    Light,
    Speaker,
    Virtual,
    Storage,
    Case,
    Microphone,
    Accessory,
    Keypad,
    Laptop,
    Monitor,
    Unknown,
}

/// Mapping from a virtual 2D matrix to physical LED indices.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatrixMap {
    pub width: usize,
    pub height: usize,
    /// Row-major map of length width*height. `None` means no LED at that cell.
    pub map: Vec<Option<usize>>,
}

/// Segment layout type for a region of LEDs.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SegmentType {
    Single,
    Linear,
    Matrix,
}

/// A logical region of LEDs on an output port.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SegmentDefinition {
    /// Stable id for this segment (used for updates & mode targeting).
    pub id: String,
    pub name: String,
    pub segment_type: SegmentType,
    /// Number of physical LEDs covered by this segment.
    pub leds_count: usize,
    /// Optional 2D matrix layout for this segment.
    pub matrix: Option<MatrixMap>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputCapabilities {
    /// Whether the user is allowed to edit segments for this output.
    ///
    /// Per spec: segments are only meaningful for `Linear` outputs (future feature),
    /// but we keep this generic for extension.
    pub editable: bool,
    /// Minimum total physical LEDs for this output (sum of segments).
    pub min_total_leds: usize,
    /// Maximum total physical LEDs for this output (sum of segments).
    pub max_total_leds: usize,
    /// Optional discrete list of allowed total LED counts (e.g. only 100 or 120).
    pub allowed_total_leds: Option<Vec<usize>>,
    /// Allowed segment types when editing is enabled.
    pub allowed_segment_types: Vec<SegmentType>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputPortDefinition {
    /// Stable id for this output port.
    pub id: String,
    pub name: String,
    /// Driver-defined output layout type (point/linear/matrix).
    pub output_type: SegmentType,
    /// Total physical LED count for this output.
    pub leds_count: usize,
    /// Optional 2D matrix layout for `Matrix` outputs.
    pub matrix: Option<MatrixMap>,
    pub capabilities: OutputCapabilities,
}

// Removed Sync, as we use Mutex to coordinate access and SerialPort is often not Sync
pub trait Controller: Send {
    fn port_name(&self) -> String;
    fn model(&self) -> String;
    fn description(&self) -> String;
    fn serial_id(&self) -> String;

    /// High-level device type (used mainly for UI grouping).
    fn device_type(&self) -> DeviceType {
        DeviceType::Light
    }

    /// Outputs exposed by this device.
    fn outputs(&self) -> Vec<OutputPortDefinition>;

    /// Update the device with a flattened frame of colors in **physical order**.
    ///
    /// The physical order is defined as: outputs in `outputs()` order, and
    /// within each output, LEDs in the driver's physical order (0..leds_count).
    fn update(&mut self, colors: &[Color]) -> Result<(), String>;

    fn clear(&mut self) -> Result<(), String> {
        // Best-effort default: clear the sum of output lengths.
        let len: usize = self.outputs().iter().map(|o| o.leds_count).sum();
        let black = vec![Color::default(); len.max(1)];
        self.update(&black)
    }

    fn disconnect(&mut self) -> Result<(), String> {
        Ok(())
    }
}

pub struct ControllerMetadata {
    pub name: &'static str,
    pub description: &'static str,
    pub probe: fn() -> Vec<Box<dyn Controller>>,
}

inventory::collect!(ControllerMetadata);
