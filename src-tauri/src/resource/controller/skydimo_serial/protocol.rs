use crate::interface::controller::Color;
use serialport::SerialPort;
use std::io::{Read, Write};
use std::time::Duration;

pub struct SkydimoSerialProtocol;

impl SkydimoSerialProtocol {
    pub fn encode_into(colors: &[Color], buffer: &mut Vec<u8>) {
        let count = colors.len();
        buffer.clear();
        buffer.reserve(6 + count * 3);

        // Header: Ada (0x41, 0x64, 0x61, 0x00)
        buffer.extend_from_slice(&[0x41, 0x64, 0x61, 0x00]);
        // Count (High, Low)
        buffer.push(((count >> 8) & 0xFF) as u8);
        buffer.push((count & 0xFF) as u8);

        for color in colors {
            buffer.push(color.r);
            buffer.push(color.g);
            buffer.push(color.b);
        }
    }

    pub fn handshake(port: &mut Box<dyn SerialPort>) -> Result<(String, String), String> {
        port.write_all(b"Moni-A").map_err(|e| e.to_string())?;

        // Wait for response
        std::thread::sleep(Duration::from_millis(50));

        let mut serial_buf: Vec<u8> = vec![0; 1024];
        match port.read(&mut serial_buf) {
            Ok(t) if t > 0 => {
                let response = &serial_buf[..t];
                let response_str = String::from_utf8_lossy(response);

                // Expected format: "Model,Serial\r\n"
                if let Some(comma_pos) = response_str.find(',') {
                    let model = response_str[..comma_pos].to_string();

                    // Extract serial (after comma, before newline)
                    let after_comma = &response_str[comma_pos + 1..];
                    let serial_part = after_comma.trim(); // Remove \r\n

                    // Convert serial to hex string to match C++ behavior if needed,
                    // or just use it as is if it's already readable.
                    // The C++ code converts the raw bytes of the serial part to hex.
                    // "std::string serial_raw = response.substr(comma_pos + 1, ...)"
                    // "oss << hex << (int)ch"
                    // So we should probably hex encode the serial part bytes.

                    let serial_hex = hex::encode(serial_part);

                    Ok((model, serial_hex.to_uppercase()))
                } else {
                    Err("Invalid response format".to_string())
                }
            }
            Ok(_) => Err("No data received".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
}
