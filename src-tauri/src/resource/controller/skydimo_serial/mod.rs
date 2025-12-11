use crate::interface::controller::{Color, Controller, ControllerMetadata, DeviceType, Zone};
use crate::resource::driver::serail_port::RateLimitedSerialPort;
use inventory;
use serialport::SerialPortType;
use std::time::Duration;

mod protocol;
use protocol::SkydimoSerialProtocol;
mod config;
use config::build_layout_from_device_name;

/// Baud rate used for Skydimo serial devices.
const BAUD_RATE: u32 = 115_200;

pub struct SkydimoSerialController {
    pub port_name: String,
    model: String,
    id: String,
    port: RateLimitedSerialPort,
    zones: Vec<Zone>,
    led_count: usize,
    buffer_cache: Vec<Color>,
    packet_cache: Vec<u8>,
}

impl SkydimoSerialController {
    fn new(
        port_name: String,
        model: String,
        id: String,
        port: RateLimitedSerialPort,
    ) -> Self {
        // Try to build a matrix layout from the reported model name.
        let (zones, led_count) = if let Some(layout) = build_layout_from_device_name(&model) {
            (vec![layout.zone], layout.total_leds)
        } else {
            // Fallback: treat as a simple linear strip of 100 LEDs.
            (vec![Zone::linear("LED Strip", 0, 100)], 100)
        };

        Self {
            port_name,
            model,
            id,
            port,
            zones,
            led_count,
            buffer_cache: Vec::with_capacity(led_count),
            packet_cache: Vec::with_capacity(led_count * 3 + 10),
        }
    }
}

impl Controller for SkydimoSerialController {
    fn port_name(&self) -> String {
        self.port_name.clone()
    }

    fn model(&self) -> String {
        self.model.clone()
    }

    fn description(&self) -> String {
        "Skydimo Serial Device".to_string()
    }

    fn serial_id(&self) -> String {
        self.id.clone()
    }

    fn length(&self) -> usize {
        self.led_count
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Light
    }

    fn zones(&self) -> Vec<Zone> {
        self.zones.clone()
    }

    fn update(&mut self, colors: &[Color]) -> Result<(), String> {
        // Ensure buffer cache is sized correctly
        if self.buffer_cache.len() != self.led_count {
            self.buffer_cache.resize(self.led_count, Color::default());
        }

        // If we have a matrix zone, map from virtual matrix buffer into the
        // physical LED order defined by the Skydimo configuration.
        if let Some(matrix_zone) = self.zones.iter().find(|z| z.matrix.is_some()) {
            let matrix = matrix_zone.matrix.as_ref().unwrap();
            let expected = matrix.width.saturating_mul(matrix.height);

            if colors.len() != expected {
                // Mismatched frame size – fall back to clamping on physical count.
                for (i, c) in colors.iter().take(self.led_count).enumerate() {
                    self.buffer_cache[i] = *c;
                }
            } else {
                // Clear buffer with black first if needed, but usually we overwrite.
                // Since the mapping might be sparse (Option<usize>), we should clear or
                // assume unmapped LEDs are black.
                // For performance, if we map *all* LEDs, we don't need to clear.
                // But let's be safe and fill with default (black) if map is sparse.
                // However, filling every frame is cost.
                // Let's just overwrite mapped ones. If unmapped ones retain old color, that might be a glitch.
                // Ideally we clear.
                self.buffer_cache.fill(Color::default());

                for (virtual_idx, opt_led) in matrix.map.iter().enumerate() {
                    if let Some(led_idx) = opt_led {
                        if *led_idx < self.buffer_cache.len() && virtual_idx < colors.len() {
                            self.buffer_cache[*led_idx] = colors[virtual_idx];
                        }
                    }
                }
            }
        } else {
            // No matrix information – treat the buffer as physical order.
            // Copy min(colors.len(), led_count)
            let len = colors.len().min(self.led_count);
            self.buffer_cache[..len].copy_from_slice(&colors[..len]);

            // If buffer is larger than input, zero out the rest?
            if len < self.led_count {
                self.buffer_cache[len..].fill(Color::default());
            }
        };

        SkydimoSerialProtocol::encode_into(&self.buffer_cache, &mut self.packet_cache);
        // Use rate-limited write; returns Ok(false) if frame was dropped due to throttling.
        self.port
            .write_all_throttled(&self.packet_cache)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn probe() -> Vec<Box<dyn Controller>> {
    let mut controllers: Vec<Box<dyn Controller>> = Vec::new();
    let ports = serialport::available_ports().unwrap_or_default();

    for p in ports {
        let is_valid = match &p.port_type {
            SerialPortType::UsbPort(info) => info.vid == 0x1A86 && info.pid == 0x7523,
            _ => false,
        };
        if !is_valid {
            continue;
        }

        if let Ok(mut port) = serialport::new(&p.port_name, BAUD_RATE)
            .timeout(Duration::from_millis(200))
            .open()
        {
            match SkydimoSerialProtocol::handshake(&mut port) {
                Ok((model, id)) => {
                    // Prepend "Skydimo" if not present, to match C++ "Skydimo " + model
                    let full_model = if !model.starts_with("Skydimo") {
                        format!("Skydimo {}", model)
                    } else {
                        model.clone()
                    };

                    // Compute frame size for rate limiting based on LED count.
                    let led_count =
                        if let Some(layout) = build_layout_from_device_name(&full_model) {
                            layout.total_leds
                        } else {
                            100 // Fallback
                        };
                    let frame_size = 6 + led_count * 3;

                    // Wrap the port in a rate-limited driver.
                    let rate_limited_port =
                        RateLimitedSerialPort::new(port, BAUD_RATE, frame_size);

                    controllers.push(Box::new(SkydimoSerialController::new(
                        p.port_name.clone(),
                        full_model,
                        id,
                        rate_limited_port,
                    )));
                }
                Err(_) => {
                    // Failed handshake, ignore device
                }
            }
        }
    }
    controllers
}

inventory::submit!(ControllerMetadata {
    name: "Skydimo Serial Controller",
    description: "Skydimo Serial LED Strip Driver",
    probe: probe,
});
