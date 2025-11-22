use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// Removed Sync, as we use Mutex to coordinate access and SerialPort is often not Sync
pub trait Controller: Send {
    fn port_name(&self) -> String;
    fn model(&self) -> String;
    fn description(&self) -> String;
    fn serial_id(&self) -> String;
    fn length(&self) -> usize;
    fn update(&mut self, colors: &[Color]) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String> {
        let len = self.length();
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
