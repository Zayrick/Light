use crate::interface::controller::{Controller, ControllerMetadata, Color, DeviceType, Zone};
use serialport::{SerialPort, SerialPortType};
use std::io::Write;
use std::time::Duration;
use inventory;

mod protocol;
use protocol::SkydimoSerialProtocol;
mod config;
use config::build_layout_from_device_name;

pub struct SkydimoSerialController {
    pub port_name: String, 
    model: String,
    id: String,
    port: Box<dyn SerialPort>,
    zones: Vec<Zone>,
    led_count: usize,
}

impl SkydimoSerialController {
    fn new(port_name: String, model: String, id: String, port: Box<dyn SerialPort>) -> Self {
        // Try to build a matrix layout from the reported model name.
        let (zones, led_count) = if let Some(layout) = build_layout_from_device_name(&model) {
            (vec![layout.zone], layout.total_leds)
        } else {
            // Fallback: treat as a simple linear strip of 100 LEDs.
            (vec![Zone::linear("LED Strip", 0, 100)], 100)
        };

        Self { port_name, model, id, port, zones, led_count }
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
        // If we have a matrix zone, map from virtual matrix buffer into the
        // physical LED order defined by the Skydimo configuration.
        let mapped: Vec<Color> = if let Some(matrix_zone) = self
            .zones
            .iter()
            .find(|z| z.matrix.is_some())
        {
            let matrix = matrix_zone.matrix.as_ref().unwrap();
            let expected = matrix.width.saturating_mul(matrix.height);

            if colors.len() != expected {
                // Mismatched frame size – fall back to clamping on physical count.
                let mut out = vec![Color::default(); self.led_count];
                for (i, c) in colors.iter().take(self.led_count).enumerate() {
                    out[i] = *c;
                }
                out
            } else {
                let mut out = vec![Color::default(); self.led_count];

                for (virtual_idx, opt_led) in matrix.map.iter().enumerate() {
                    if let Some(led_idx) = opt_led {
                        if *led_idx < out.len() && virtual_idx < colors.len() {
                            out[*led_idx] = colors[virtual_idx];
                        }
                    }
                }

                out
            }
        } else {
            // No matrix information – treat the buffer as physical order.
            let mut out = vec![Color::default(); self.led_count];
            for (i, c) in colors.iter().take(self.led_count).enumerate() {
                out[i] = *c;
            }
            out
        };

        let packet = SkydimoSerialProtocol::encode_frame(&mapped);
        self.port.write_all(&packet).map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn probe() -> Vec<Box<dyn Controller>> {
    let mut controllers: Vec<Box<dyn Controller>> = Vec::new();
    let ports = serialport::available_ports().unwrap_or_default();

    for p in ports {
        let is_usb = match &p.port_type {
            SerialPortType::UsbPort(_) => true,
            _ => false,
        };
        if !is_usb { continue; }

        if let Ok(mut port) = serialport::new(&p.port_name, 115_200)
            .timeout(Duration::from_millis(200))
            .open() 
        {
             match SkydimoSerialProtocol::handshake(&mut port) {
                 Ok((model, id)) => {
                     // Prepend "Skydimo" if not present, to match C++ "Skydimo " + model
                     let full_model = if !model.starts_with("Skydimo") {
                         format!("Skydimo {}", model)
                     } else {
                         model
                     };

                     controllers.push(Box::new(SkydimoSerialController::new(
                         p.port_name.clone(), 
                         full_model, 
                         id, 
                         port
                     )));
                 },
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
