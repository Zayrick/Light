use crate::interface::controller::{
    Color, Controller, ControllerMetadata, DeviceType, OutputCapabilities, OutputPortDefinition,
    SegmentType,
};
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
    outputs: Vec<OutputPortDefinition>,
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
        // Try to build a default layout from the reported model name.
        let (output_type, led_count, matrix) = if let Some(layout) = build_layout_from_device_name(&model) {
            (layout.segment_type, layout.total_leds, layout.matrix)
        } else {
            // Fallback: treat as a simple linear strip of 100 LEDs.
            (SegmentType::Linear, 100, None)
        };

        let capabilities = match output_type {
            SegmentType::Matrix => OutputCapabilities {
                editable: false,
                min_total_leds: led_count,
                max_total_leds: led_count,
                allowed_total_leds: Some(vec![led_count]),
                allowed_segment_types: vec![SegmentType::Matrix],
            },
            SegmentType::Linear | SegmentType::Single => OutputCapabilities {
                // Allow segment editing, but keep total LED count fixed for this controller.
                editable: true,
                min_total_leds: led_count,
                max_total_leds: led_count,
                allowed_total_leds: Some(vec![led_count]),
                allowed_segment_types: vec![
                    SegmentType::Single,
                    SegmentType::Linear,
                    SegmentType::Matrix,
                ],
            },
        };

        let outputs = vec![OutputPortDefinition {
            id: "out1".to_string(),
            name: "Output 1".to_string(),
            output_type,
            leds_count: led_count,
            matrix,
            capabilities,
        }];

        Self {
            port_name,
            model,
            id,
            port,
            outputs,
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

    fn device_type(&self) -> DeviceType {
        DeviceType::Light
    }

    fn outputs(&self) -> Vec<OutputPortDefinition> {
        self.outputs.clone()
    }

    fn update(&mut self, colors: &[Color]) -> Result<(), String> {
        // Ensure buffer cache is sized correctly
        if self.buffer_cache.len() != self.led_count {
            self.buffer_cache.resize(self.led_count, Color::default());
        }

        // Treat the input buffer as **physical LED order**.
        let len = colors.len().min(self.led_count);
        self.buffer_cache[..len].copy_from_slice(&colors[..len]);
        if len < self.led_count {
            self.buffer_cache[len..].fill(Color::default());
        }

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
                    let led_count = if let Some(layout) = build_layout_from_device_name(&full_model) {
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
    probe,
});
