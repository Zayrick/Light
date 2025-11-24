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

/// Zone layout type for a region of LEDs.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ZoneType {
    Single,
    Linear,
    Matrix,
}

/// Mapping from a virtual 2D matrix to physical LED indices.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatrixMap {
    pub width: usize,
    pub height: usize,
    /// Row-major map of length width*height. `None` means no LED at that cell.
    pub map: Vec<Option<usize>>,
}

/// A logical region of LEDs on a device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub name: String,
    pub zone_type: ZoneType,
    /// Starting physical LED index covered by this zone (if linear/single).
    pub start_index: usize,
    /// Number of physical LEDs covered by this zone.
    pub leds_count: usize,
    /// Optional 2D matrix layout for this zone.
    pub matrix: Option<MatrixMap>,
}

impl Zone {
    pub fn single(name: impl Into<String>, index: usize) -> Self {
        Self {
            name: name.into(),
            zone_type: ZoneType::Single,
            start_index: index,
            leds_count: 1,
            matrix: None,
        }
    }

    pub fn linear(name: impl Into<String>, start_index: usize, leds_count: usize) -> Self {
        Self {
            name: name.into(),
            zone_type: ZoneType::Linear,
            start_index,
            leds_count,
            matrix: None,
        }
    }

    pub fn matrix(name: impl Into<String>, matrix: MatrixMap, total_leds: usize) -> Self {
        Self {
            name: name.into(),
            zone_type: ZoneType::Matrix,
            start_index: 0,
            leds_count: total_leds,
            matrix: Some(matrix),
        }
    }
}

// Removed Sync, as we use Mutex to coordinate access and SerialPort is often not Sync
pub trait Controller: Send {
    fn port_name(&self) -> String;
    fn model(&self) -> String;
    fn description(&self) -> String;
    fn serial_id(&self) -> String;

    /// Number of physical LEDs on the device.
    fn length(&self) -> usize;

    /// High-level device type (used mainly for UI grouping).
    fn device_type(&self) -> DeviceType {
        DeviceType::Light
    }

    /// Logical zones for this device. Defaults to a single linear zone.
    fn zones(&self) -> Vec<Zone> {
        vec![Zone::linear("Zone 1", 0, self.length())]
    }

    /// Virtual 2D layout (width, height) used by effects.
    ///
    /// By default, if a matrix zone exists, its dimensions are used; otherwise,
    /// the device is treated as a 1D strip with height 1.
    fn virtual_layout(&self) -> (usize, usize) {
        for zone in self.zones() {
            if zone.zone_type == ZoneType::Matrix {
                if let Some(matrix) = &zone.matrix {
                    return (matrix.width, matrix.height);
                }
            }
        }
        (self.length(), 1)
    }

    /// Update the device with a frame of colors in virtual layout order.
    fn update(&mut self, colors: &[Color]) -> Result<(), String>;

    fn clear(&mut self) -> Result<(), String> {
        let (w, h) = self.virtual_layout();
        let len = w
            .checked_mul(h)
            .unwrap_or(0)
            .max(1);
        let black = vec![Color::default(); len];
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
