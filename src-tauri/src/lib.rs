use std::time::Duration;
use std::thread;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use serialport::SerialPortType;
use tauri::State;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(serde::Serialize)]
struct Device {
    port: String,
    model: String,
    id: String,
}

struct AppState {
    // Map port name to a "keep running" flag for the animation thread
    running_loops: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

fn build_packet(led_data: &[u8]) -> Vec<u8> {
    // PACKET = bytearray.fromhex(f'41646100{LEDS:04x}') + led_data
    // 41=A, 64=d, 61=a
    // Header: A d a 0x00 [Count_Hi] [Count_Lo]
    
    let count = led_data.len() / 3; // Assumes RGB
    let mut packet = Vec::new();
    packet.push(0x41); // 'A'
    packet.push(0x64); // 'd'
    packet.push(0x61); // 'a'
    packet.push(0x00); 
    packet.push(((count >> 8) & 0xFF) as u8);
    packet.push((count & 0xFF) as u8);
    packet.extend_from_slice(led_data);
    packet
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

#[tauri::command]
fn set_rainbow(port: String, state: State<AppState>) -> Result<(), String> {
    let mut runners = state.running_loops.lock().unwrap();
    
    // Stop any existing animation on this port
    if let Some(flag) = runners.get(&port) {
        flag.store(false, Ordering::Relaxed);
    }
    
    // Create a new flag for the new animation thread
    let running_flag = Arc::new(AtomicBool::new(true));
    runners.insert(port.clone(), running_flag.clone());
    
    let port_name = port.clone();
    
    // Spawn the animation thread
    thread::spawn(move || {
        // Attempt to open the serial port
        // Note: 115200 baud allows ~11520 bytes/sec. 
        // 100 LEDs = 300 bytes + header. 
        // Max FPS is approx 37 FPS. 60 FPS might cause buffering/lag, but we try to send fast.
        let mut serial_port = match serialport::new(&port_name, 115_200)
            .timeout(Duration::from_secs(1))
            .open() 
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to open port {} for animation: {}", port_name, e);
                return;
            }
        };

        let led_count = 100;
        let mut offset = 0.0;

        while running_flag.load(Ordering::Relaxed) {
            let mut led_data = Vec::with_capacity(led_count * 3);

            for i in 0..led_count {
                // Spread hue across 360 degrees, shift by offset for animation
                let hue = ((i as f32 * 360.0 / led_count as f32) + offset) % 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
                led_data.push(r);
                led_data.push(g);
                led_data.push(b);
            }

            let packet = build_packet(&led_data);
            
            if let Err(e) = serial_port.write_all(&packet) {
                eprintln!("Write error on {}: {}", port_name, e);
                break;
            }

            // Update animation state
            offset += 2.0; 
            if offset >= 360.0 {
                offset -= 360.0;
            }

            // Target ~60 FPS (16ms)
            thread::sleep(Duration::from_millis(16));
        }
        
        // Port closes automatically when dropped here
    });

    Ok(())
}

#[tauri::command]
fn turn_off(port: String, state: State<AppState>) -> Result<(), String> {
    // 1. Signal any running thread to stop
    {
        let mut runners = state.running_loops.lock().unwrap();
        if let Some(flag) = runners.remove(&port) {
            flag.store(false, Ordering::Relaxed);
        }
    }

    // 2. Try to open the port and send black. 
    // We might need to retry a few times if the animation thread is still holding the lock.
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(2);
    
    loop {
        if start.elapsed() > timeout {
            return Err("Timeout waiting for port to become available".into());
        }

        match serialport::new(&port, 115_200)
            .timeout(Duration::from_millis(200))
            .open() 
        {
            Ok(mut serial) => {
                let led_count = 100;
                let led_data = vec![0u8; led_count * 3]; 
                let packet = build_packet(&led_data);
                serial.write_all(&packet).map_err(|e| e.to_string())?;
                return Ok(());
            },
            Err(_) => {
                // Port might be busy, wait a bit
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

#[tauri::command]
fn scan_devices() -> Vec<Device> {
    let mut devices = Vec::new();
    let ports = serialport::available_ports().unwrap_or_default();

    for p in ports {
        // Python: if "USB-SERIAL" not in p.description: continue
        // In Rust serialport, we can check port_type
        let is_usb = match &p.port_type {
            SerialPortType::UsbPort(_) => true,
             _ => false, // The python script relies on description string which might vary. 
                        // But generally we only care about USB serial devices for this hardware.
        };
        
        // If strict filtering is needed we can look at p.port_name or p.port_type
        // For now, let's try to open it if it is a USB port or if the description matches (if we could access it easily like python)
        // serialport-rs doesn't expose 'description' exactly like pyserial, but usually UsbPort covers it.
        
        // NOTE: Trying to open all ports can be slow/risky, so we try to stick to USB ones.
        if !is_usb {
             continue;
        }

        if let Ok(mut port) = serialport::new(&p.port_name, 115_200)
            .timeout(Duration::from_secs(1))
            .open() 
        {
            // Handshake
            if port.write_all(b"Moni-A").is_ok() {
                thread::sleep(Duration::from_millis(100));
                
                // Read response
                let mut serial_buf: Vec<u8> = vec![0; 1024];
                if let Ok(t) = port.read(&mut serial_buf) {
                    let response = &serial_buf[..t];
                    let response_hex = hex::encode(response);
                    
                    // Check for '2c' (comma)
                    if response_hex.contains("2c") {
                        let parts: Vec<&str> = response_hex.splitn(2, "2c").collect();
                        if parts.len() == 2 {
                            let model_hex = parts[0];
                            let id_part = parts[1];
                            
                            // Decode model
                            let model = match hex::decode(model_hex) {
                                Ok(bytes) => String::from_utf8_lossy(&bytes).to_string().to_uppercase(),
                                Err(_) => "Unknown".to_string(),
                            };
                            
                            // Clean ID: remove 0d0a (\r\n)
                            let id = id_part.to_uppercase().replace("0D0A", "");
                            
                            devices.push(Device {
                                port: p.port_name.clone(),
                                model,
                                id,
                            });
                        }
                    }
                }
            }
        }
    }
    
    devices
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            running_loops: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![greet, scan_devices, set_rainbow, turn_off])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
