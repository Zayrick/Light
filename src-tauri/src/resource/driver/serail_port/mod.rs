//! Rate-limited serial port driver layer.
//!
//! This module provides a wrapper around `serialport::SerialPort` that automatically
//! throttles writes based on baud rate and payload size, preventing buffer overflow
//! issues on macOS and other platforms where the OS serial buffer is less forgiving.

use serialport::SerialPort;
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// A rate-limited serial port wrapper that automatically throttles writes
/// to prevent overflowing the device's receive buffer.
///
/// # Rate Limiting Strategy
/// - Computes a safe frame interval based on frame size and baud rate
/// - Drops intermediate frames if called faster than the computed interval
/// - Uses conservative calculation: `floor(theoretical_fps) - 1` (minimum 1 FPS)
///
/// # Formula
/// ```text
/// frame_bytes = header_size + payload_size
/// theoretical_fps = (baud_rate / bits_per_byte) / frame_bytes
/// safe_fps = max(floor(theoretical_fps) - 1, 1)
/// min_interval = 1 / safe_fps
/// ```
pub struct RateLimitedSerialPort {
    port: Box<dyn SerialPort>,
    baud_rate: u32,
    min_interval: Duration,
    last_send: Option<Instant>,
}

impl RateLimitedSerialPort {
    /// Creates a new rate-limited serial port wrapper.
    ///
    /// # Arguments
    /// * `port` - The underlying serial port
    /// * `baud_rate` - The baud rate of the serial connection
    /// * `frame_size` - The expected frame size in bytes (header + payload)
    ///
    /// # Example
    /// ```ignore
    /// let port = serialport::new("/dev/ttyUSB0", 115_200).open()?;
    /// let frame_size = 6 + led_count * 3; // header + RGB data
    /// let rate_limited = RateLimitedSerialPort::new(port, 115_200, frame_size);
    /// ```
    pub fn new(port: Box<dyn SerialPort>, baud_rate: u32, frame_size: usize) -> Self {
        let min_interval = Self::compute_min_interval(baud_rate, frame_size);
        Self {
            port,
            baud_rate,
            min_interval,
            last_send: None,
        }
    }

    /// Computes the minimum interval between frames based on baud rate and frame size.
    ///
    /// Uses a conservative calculation:
    /// - Each byte on UART is ~10 bits (1 start + 8 data + 1 stop)
    /// - Safe FPS = floor(theoretical_fps) - 1, minimum 1 FPS
    fn compute_min_interval(baud_rate: u32, frame_size: usize) -> Duration {
        const BITS_PER_BYTE: f64 = 10.0;
        let bytes_per_second = baud_rate as f64 / BITS_PER_BYTE;
        let theoretical_fps = bytes_per_second / frame_size as f64;
        let safe_fps = (theoretical_fps.floor() - 1.0).max(1.0);
        Duration::from_secs_f64(1.0 / safe_fps)
    }

    /// Updates the frame size and recalculates the minimum interval.
    ///
    /// Call this if the payload size changes dynamically.
    pub fn set_frame_size(&mut self, frame_size: usize) {
        self.min_interval = Self::compute_min_interval(self.baud_rate, frame_size);
    }

    /// Returns the current computed safe FPS.
    pub fn safe_fps(&self) -> f64 {
        1.0 / self.min_interval.as_secs_f64()
    }

    /// Returns the minimum interval between frames.
    pub fn min_interval(&self) -> Duration {
        self.min_interval
    }

    /// Writes data to the serial port with rate limiting.
    ///
    /// If called within the minimum interval since the last successful write,
    /// this method returns `Ok(0)` without writing, effectively dropping the frame.
    ///
    /// # Returns
    /// - `Ok(bytes_written)` - Number of bytes written (0 if frame was dropped)
    /// - `Err(e)` - IO error from the underlying serial port
    pub fn write_throttled(&mut self, data: &[u8]) -> io::Result<usize> {
        let now = Instant::now();

        // Check if we're still within the rate limit interval
        if let Some(last) = self.last_send {
            if now.duration_since(last) < self.min_interval {
                // Drop this frame - we're sending too fast
                return Ok(0);
            }
        }

        // Write the data
        let bytes_written = self.port.write(data)?;
        self.last_send = Some(now);
        Ok(bytes_written)
    }

    /// Writes all data to the serial port with rate limiting.
    ///
    /// Similar to `write_throttled`, but ensures all data is written if not rate-limited.
    ///
    /// # Returns
    /// - `Ok(true)` - Data was written successfully
    /// - `Ok(false)` - Frame was dropped due to rate limiting
    /// - `Err(e)` - IO error from the underlying serial port
    pub fn write_all_throttled(&mut self, data: &[u8]) -> io::Result<bool> {
        let now = Instant::now();

        // Check if we're still within the rate limit interval
        if let Some(last) = self.last_send {
            if now.duration_since(last) < self.min_interval {
                // Drop this frame - we're sending too fast
                return Ok(false);
            }
        }

        // Write all data
        self.port.write_all(data)?;
        self.last_send = Some(now);
        Ok(true)
    }

    /// Returns a mutable reference to the underlying serial port.
    ///
    /// Use this for operations that don't need rate limiting (e.g., handshake, configuration).
    pub fn inner_mut(&mut self) -> &mut Box<dyn SerialPort> {
        &mut self.port
    }

    /// Returns a reference to the underlying serial port.
    pub fn inner(&self) -> &dyn SerialPort {
        &*self.port
    }

    /// Consumes this wrapper and returns the underlying serial port.
    pub fn into_inner(self) -> Box<dyn SerialPort> {
        self.port
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_min_interval() {
        // 100 LEDs: frame_size = 6 + 100 * 3 = 306 bytes
        // At 115200 baud: theoretical = 11520 / 306 ≈ 37.6 FPS
        // Safe FPS = floor(37.6) - 1 = 36 FPS
        let interval = RateLimitedSerialPort::compute_min_interval(115_200, 306);
        let fps = 1.0 / interval.as_secs_f64();
        assert!((fps - 36.0).abs() < 0.1, "Expected ~36 FPS, got {}", fps);
    }

    #[test]
    fn test_compute_min_interval_small_frame() {
        // 10 LEDs: frame_size = 6 + 10 * 3 = 36 bytes
        // At 115200 baud: theoretical = 11520 / 36 = 320 FPS
        // Safe FPS = floor(320) - 1 = 319 FPS
        let interval = RateLimitedSerialPort::compute_min_interval(115_200, 36);
        let fps = 1.0 / interval.as_secs_f64();
        assert!((fps - 319.0).abs() < 0.1, "Expected ~319 FPS, got {}", fps);
    }

    #[test]
    fn test_compute_min_interval_large_frame() {
        // 500 LEDs: frame_size = 6 + 500 * 3 = 1506 bytes
        // At 115200 baud: theoretical = 11520 / 1506 ≈ 7.65 FPS
        // Safe FPS = floor(7.65) - 1 = 6 FPS
        let interval = RateLimitedSerialPort::compute_min_interval(115_200, 1506);
        let fps = 1.0 / interval.as_secs_f64();
        assert!((fps - 6.0).abs() < 0.1, "Expected ~6 FPS, got {}", fps);
    }
}
