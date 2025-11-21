use crate::interface::controller::{Controller, ControllerMetadata, Color};
use serialport::{SerialPort, SerialPortType};
use std::time::Duration;
use std::io::{Read, Write};
use inventory;

pub struct MoniAController {
    pub port_name: String, 
    model: String,
    id: String,
    port: Box<dyn SerialPort>,
}

impl MoniAController {
    fn new(port_name: String, model: String, id: String, port: Box<dyn SerialPort>) -> Self {
        Self { port_name, model, id, port }
    }
}

impl Controller for MoniAController {
    fn port_name(&self) -> String {
        self.port_name.clone()
    }

    fn model(&self) -> String {
        self.model.clone()
    }

    fn serial_id(&self) -> String {
        self.id.clone()
    }

    fn update(&mut self, colors: &[Color]) -> Result<(), String> {
        let count = colors.len();
        let mut packet = Vec::new();
        packet.push(0x41); 
        packet.push(0x64); 
        packet.push(0x61); 
        packet.push(0x00); 
        packet.push(((count >> 8) & 0xFF) as u8);
        packet.push((count & 0xFF) as u8);
        
        for color in colors {
            packet.push(color.r);
            packet.push(color.g);
            packet.push(color.b);
        }
        
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
             if port.write_all(b"Moni-A").is_ok() {
                std::thread::sleep(Duration::from_millis(100));
                let mut serial_buf: Vec<u8> = vec![0; 1024];
                if let Ok(t) = port.read(&mut serial_buf) {
                    let response = &serial_buf[..t];
                    let response_hex = hex::encode(response);
                    
                    if response_hex.contains("2c") {
                        let parts: Vec<&str> = response_hex.splitn(2, "2c").collect();
                        if parts.len() == 2 {
                            let model_hex = parts[0];
                            let id_part = parts[1];
                            
                            let model = match hex::decode(model_hex) {
                                Ok(bytes) => String::from_utf8_lossy(&bytes).to_string().to_uppercase(),
                                Err(_) => "Unknown".to_string(),
                            };
                            
                            let id = id_part.to_uppercase().replace("0D0A", "");
                            
                            controllers.push(Box::new(MoniAController::new(p.port_name.clone(), model, id, port)));
                        }
                    }
                }
             }
        }
    }
    controllers
}

inventory::submit!(ControllerMetadata {
    name: "Moni-A Controller",
    description: "Generic Moni-A Serial Protocol Device",
    probe: probe,
});

