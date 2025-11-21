use crate::interface::controller::{Controller, ControllerMetadata, Color};
use serialport::{SerialPort, SerialPortType};
use std::time::Duration;
use inventory;

mod protocol;
use protocol::MoniAProtocol;

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

    fn length(&self) -> usize {
        // Default to 100 as per original, or could be dynamic if protocol supported it
        100 
    }

    fn update(&mut self, colors: &[Color]) -> Result<(), String> {
        let packet = MoniAProtocol::encode_frame(colors);
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
             match MoniAProtocol::handshake(&mut port) {
                 Ok((model, id)) => {
                     // Prepend "Skydimo" if not present, to match C++ "Skydimo " + model
                     let full_model = if !model.starts_with("Skydimo") {
                         format!("Skydimo {}", model)
                     } else {
                         model
                     };

                     controllers.push(Box::new(MoniAController::new(
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
    name: "Skydimo Controller",
    description: "Skydimo Serial LED Strip Driver",
    probe: probe,
});

