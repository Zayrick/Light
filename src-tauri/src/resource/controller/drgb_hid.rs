use hidapi::{HidApi, HidDevice};
use inventory;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use crate::interface::controller::{
    Color, Controller, ControllerMetadata, DeviceType, OutputCapabilities, OutputPortDefinition,
    SegmentType,
};

const DRGBV4_VID: u16 = 0x2486;
const DRGB_LED_V4_PID: u16 = 0x3608;
const DRGB_ULTRA_V4F_PID: u16 = 0x3616;
const DRGB_CORE_V4F_PID: u16 = 0x3628;
const DRGB_SIG_V4F_PID: u16 = 0x3636;
const DRGB_AG_04_V4F_PID: u16 = 0x3204;
const DRGB_AG_16_V4F_PID: u16 = 0x3216;

const DRGB_LED_V5_PID: u16 = 0x3208;
const DRGB_ULTRA_V5_PID: u16 = 0x3215;
const DRGB_ULTRA_V5F_PID: u16 = 0x3217;
const DRGB_CORE_V5_PID: u16 = 0x3228;
const DRGB_CORE_V5F_PID: u16 = 0x3229;
const DRGB_SIG_V5F_PID: u16 = 0x3232;

const DRGBV3_VID: u16 = 0x2023;
const DRGB_LED_V3_PID: u16 = 0x1209;
const DRGB_ULTRA_V3_PID: u16 = 0x1221;
const DRGB_CORE_V3_PID: u16 = 0x1226;
const DRGB_ELITE_PID: u16 = 0x1408;
const DM_10_PID: u16 = 0x1410;
const JPU_12_PID: u16 = 0x1412;

const DRGBV2_VID: u16 = 0x2023;
const DRGB_LED_PID: u16 = 0x1208;
const DRGB_ULTRA_PID: u16 = 0x1220;
const DRGB_SIG_AB_PID: u16 = 0x1210;
const DRGB_SIG_CD_PID: u16 = 0x1211;
const DRGB_STRIMER_PID: u16 = 0x1215;

const YICO_VID: u16 = 0x1368;
const YICO_8_PID: u16 = 0x6077;
const YICO_08_PID: u16 = 0x6078;
const YICO_08_1_PID: u16 = 0x6079;

const DRGB_V4_ONE_PACKAGE_SIZE: usize = 316;
const DRGB_V4_PACKAGE_SIZE: usize = 340;

// V3/V1 share the same 21-LED packet framing in OpenRGB (21 * 3 = 63 bytes payload)
const DRGB_V3_PACKAGE_SIZE: usize = 21;
// V2 uses 20 LEDs per packet (20 * 3 = 60 bytes payload)
const DRGB_V2_PACKAGE_SIZE: usize = 20;

struct DrgbConfig {
    name: &'static str,
    num_channels: usize,
    leds_per_channel: usize,
    version: u8,
}

fn drgb_output_name(num_channels: usize, channel_idx: usize) -> String {
    // Mirrors tmp\DRGBController\RGBController_DRGB.cpp SetupZones naming.
    // Note: OpenRGB appends the numeric suffix for all zones, including Strimer names.
    if num_channels == 6 {
        if channel_idx == 0 {
            return format!("Strimer ATX{}", channel_idx + 1);
        }
        if channel_idx < 3 {
            // Channel C1..2
            return format!("Channel C{}", channel_idx);
        }
        if channel_idx == 3 {
            // Strimer GPU1
            return "Strimer GPU1".to_string();
        }
        if channel_idx < 6 {
            // Channel D1..2
            return format!("Channel D{}", channel_idx - 3);
        }
    }

    if num_channels == 10 || num_channels == 12 {
        return format!("Channel {}", channel_idx + 1);
    }

    if channel_idx < 8 {
        return format!("Channel A{}", channel_idx + 1);
    }

    if channel_idx < 16 {
        return format!("Channel B{}", channel_idx - 7);
    }

    if num_channels == 30 {
        if channel_idx < 24 {
            return format!("Channel C{}", channel_idx - 15);
        }
        if channel_idx < 30 {
            return format!("Channel D{}", channel_idx - 23);
        }
    }

    if channel_idx < 22 {
        return format!("Channel C{}", channel_idx - 15);
    }

    if channel_idx < 28 {
        return format!("Channel D{}", channel_idx - 21);
    }

    if channel_idx < 36 {
        return format!("Channel E{}", channel_idx - 27);
    }

    // Fallback (shouldn't happen with known configs)
    format!("Channel {}", channel_idx + 1)
}

fn get_drgb_config(pid: u16) -> Option<DrgbConfig> {
    match pid {
        DRGB_LED_V4_PID => Some(DrgbConfig { name: "DRGB LED V4", num_channels: 8, leds_per_channel: 10, version: 4 }),
        DRGB_ULTRA_V4F_PID => Some(DrgbConfig { name: "DRGB ULTRA V4F", num_channels: 16, leds_per_channel: 10, version: 4 }),
        DRGB_CORE_V4F_PID => Some(DrgbConfig { name: "DRGB CORE V4F", num_channels: 32, leds_per_channel: 10, version: 4 }),
        DRGB_SIG_V4F_PID => Some(DrgbConfig { name: "DRGB SIG V4F", num_channels: 36, leds_per_channel: 10, version: 4 }),
        DRGB_AG_04_V4F_PID => Some(DrgbConfig { name: "Airgoo AG-DRGB04", num_channels: 4, leds_per_channel: 10, version: 4 }),
        DRGB_AG_16_V4F_PID => Some(DrgbConfig { name: "Airgoo AG-DRGB16", num_channels: 16, leds_per_channel: 10, version: 4 }),
        
        DRGB_LED_V5_PID => Some(DrgbConfig { name: "DRGB LED V5", num_channels: 8, leds_per_channel: 10, version: 4 }), // Assuming V4 protocol for V5 based on OpenRGB code using same Detect function
        DRGB_ULTRA_V5_PID => Some(DrgbConfig { name: "DRGB ULTRA V5", num_channels: 16, leds_per_channel: 10, version: 4 }),
        DRGB_ULTRA_V5F_PID => Some(DrgbConfig { name: "DRGB ULTRA V5F", num_channels: 16, leds_per_channel: 10, version: 4 }),
        DRGB_CORE_V5_PID => Some(DrgbConfig { name: "DRGB CORE V5", num_channels: 32, leds_per_channel: 10, version: 4 }),
        DRGB_CORE_V5F_PID => Some(DrgbConfig { name: "DRGB CORE V5F", num_channels: 32, leds_per_channel: 10, version: 4 }),
        DRGB_SIG_V5F_PID => Some(DrgbConfig { name: "DRGB SIG V5F", num_channels: 32, leds_per_channel: 10, version: 4 }),

        // V3
        DRGB_LED_V3_PID => Some(DrgbConfig { name: "DRGB LED V3", num_channels: 8, leds_per_channel: 10, version: 3 }),
        DRGB_ULTRA_V3_PID => Some(DrgbConfig { name: "DRGB Ultra V3", num_channels: 16, leds_per_channel: 10, version: 3 }),
        DRGB_CORE_V3_PID => Some(DrgbConfig { name: "DRGB CORE V3", num_channels: 30, leds_per_channel: 10, version: 3 }),

        // V1
        DRGB_ELITE_PID => Some(DrgbConfig { name: "DRGB ELITE", num_channels: 8, leds_per_channel: 10, version: 1 }),
        DM_10_PID => Some(DrgbConfig { name: "NEEDMAX 10 ELITE", num_channels: 10, leds_per_channel: 10, version: 1 }),
        JPU_12_PID => Some(DrgbConfig { name: "JPU ELITE", num_channels: 12, leds_per_channel: 10, version: 1 }),

        // V2
        DRGB_LED_PID => Some(DrgbConfig { name: "DRGB LED Controller", num_channels: 8, leds_per_channel: 10, version: 2 }),
        DRGB_ULTRA_PID => Some(DrgbConfig { name: "DRGB ULTRA", num_channels: 16, leds_per_channel: 10, version: 2 }),
        DRGB_SIG_AB_PID => Some(DrgbConfig { name: "DRGB SIG AB", num_channels: 16, leds_per_channel: 10, version: 2 }),
        DRGB_SIG_CD_PID => Some(DrgbConfig { name: "DRGB SIG CD", num_channels: 6, leds_per_channel: 10, version: 2 }),
        DRGB_STRIMER_PID => Some(DrgbConfig { name: "DRGB Strimer Controller", num_channels: 6, leds_per_channel: 10, version: 2 }),

        // YICO (uses V3 protocol in OpenRGB)
        YICO_8_PID => Some(DrgbConfig { name: "YICO 8 ELITE", num_channels: 8, leds_per_channel: 10, version: 3 }),
        YICO_08_PID => Some(DrgbConfig { name: "YICO 08 ELITE", num_channels: 8, leds_per_channel: 10, version: 3 }),
        YICO_08_1_PID => Some(DrgbConfig { name: "YICO 08 ELITE", num_channels: 8, leds_per_channel: 10, version: 3 }),

        _ => None,
    }
}

struct DrgbHidController {
    device: Arc<Mutex<HidDevice>>,
    config: DrgbConfig,
    serial: String,
    path: String,

    keepalive_run: Arc<AtomicBool>,
    last_commit: Arc<Mutex<Instant>>,
    keepalive_handle: Option<thread::JoinHandle<()>>,
}

impl DrgbHidController {
    fn new(device: HidDevice, config: DrgbConfig, serial: String, path: String) -> Self {
        let device = Arc::new(Mutex::new(device));
        let keepalive_run = Arc::new(AtomicBool::new(true));
        let last_commit = Arc::new(Mutex::new(Instant::now()));

        // Mirrors DRGBController::KeepaliveThread in OpenRGB:
        // every 500ms, if >1s since last commit, send 0x65 keepalive packet.
        let ka_device = Arc::clone(&device);
        let ka_run = Arc::clone(&keepalive_run);
        let ka_last = Arc::clone(&last_commit);
        let keepalive_handle = Some(thread::spawn(move || {
            let sleep = Duration::from_millis(500);
            while ka_run.load(Ordering::Relaxed) {
                let should_send = {
                    let last = ka_last.lock();
                    match last {
                        Ok(last) => last.elapsed() > Duration::from_secs(1),
                        Err(_) => true,
                    }
                };

                if should_send {
                    if let Ok(dev) = ka_device.lock() {
                        // Equivalent to SendPacketFS(sleep_buf, 1, 0) with sleep_buf[0]=0x65.
                        let mut buf = [0u8; 65];
                        buf[0] = 0x00;
                        buf[1] = 0x65;
                        let _ = dev.write(&buf);
                    }
                }

                thread::sleep(sleep);
            }
        }));

        Self {
            device,
            config,
            serial,
            path,

            keepalive_run,
            last_commit,
            keepalive_handle,
        }
    }

    fn total_leds(&self) -> usize {
        self.config.num_channels * self.config.leds_per_channel
    }

    fn build_zone_ordered_rgb_bytes(&self, colors: &[Color]) -> Vec<u8> {
        // Colors are already in physical order: outputs in outputs() order, then 0..leds_count.
        // For our controller, outputs are channels 0..N with fixed leds_per_channel.
        let mut out = Vec::with_capacity(colors.len() * 3);
        for c in colors {
            out.push(c.r);
            out.push(c.g);
            out.push(c.b);
        }
        out
    }

    fn send_packet_v4(&self, device: &HidDevice, rgb_data: &[u8], led_total: usize) -> Result<(), String> {
        // Replicates DRGBController::SendPacket (OpenRGB)
        let buf_packets = if led_total > DRGB_V4_ONE_PACKAGE_SIZE {
            1 + ((led_total - DRGB_V4_ONE_PACKAGE_SIZE) as f32 / DRGB_V4_PACKAGE_SIZE as f32).ceil() as usize
        } else {
            1
        };

        let mut current_led_total = led_total;
        let mut hig_count: u8 = if current_led_total / 256 >= 1 { 1 } else { 0 };
        let mut low_count: u8 = if current_led_total >= DRGB_V4_ONE_PACKAGE_SIZE {
            60
        } else {
            (current_led_total % 256) as u8
        };

        current_led_total = current_led_total.saturating_sub(DRGB_V4_ONE_PACKAGE_SIZE);

        for i in 0..buf_packets {
            let mut usb_buf = [0u8; 1025];
            usb_buf[0] = 0x00; // Report ID
            usb_buf[1] = (i + 100) as u8;
            usb_buf[2] = (buf_packets + 99) as u8;
            usb_buf[3] = hig_count;
            usb_buf[4] = low_count;

            let buf_idx = i * 1020;
            for k in 0..1020 {
                usb_buf[k + 5] = rgb_data.get(buf_idx + k).copied().unwrap_or(0);
            }

            device.write(&usb_buf).map_err(|e| e.to_string())?;

            if current_led_total > 0 {
                hig_count = if current_led_total / 256 >= 1 { 1 } else { 0 };
                low_count = if current_led_total >= DRGB_V4_PACKAGE_SIZE {
                    84
                } else {
                    (current_led_total % 256) as u8
                };

                current_led_total = current_led_total.saturating_sub(DRGB_V4_PACKAGE_SIZE);
            }
        }

        Ok(())
    }

    fn send_packet_fs(&self, device: &HidDevice, payload: &[u8], buf_packets: usize, array: u8) -> Result<(), String> {
        // Replicates DRGBController::SendPacketFS (OpenRGB)
        if array == 0x64 {
            for i in 0..buf_packets {
                let mut usb_buf = [0u8; 65];
                usb_buf[0] = 0x00;
                usb_buf[1] = if i == buf_packets.saturating_sub(1) {
                    array.wrapping_add(100u8).wrapping_add(i as u8)
                } else {
                    array.wrapping_add(i as u8)
                };

                let buf_idx = i * 63;
                for k in 0..63 {
                    usb_buf[k + 2] = payload.get(buf_idx + k).copied().unwrap_or(0);
                }

                device.write(&usb_buf).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }

        if array == 0x47 {
            for i in 0..buf_packets {
                let mut usb_buf = [0u8; 65];
                usb_buf[0] = 0x00;
                usb_buf[1] = if i == buf_packets.saturating_sub(1) {
                    array.wrapping_add(92u8).wrapping_add(i as u8)
                } else {
                    array.wrapping_add(i as u8)
                };

                let buf_idx = i * 63;
                for k in 0..63 {
                    usb_buf[k + 2] = payload.get(buf_idx + k).copied().unwrap_or(0);
                }

                device.write(&usb_buf).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }

        // array == 0: write 64 bytes directly into report payload
        let mut usb_buf = [0u8; 65];
        usb_buf[0] = 0x00;
        for i in 0..64 {
            usb_buf[i + 1] = payload.get(i).copied().unwrap_or(0);
        }
        device.write(&usb_buf).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn stop_keepalive(&mut self) {
        self.keepalive_run.store(false, Ordering::Relaxed);
        if let Some(handle) = self.keepalive_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for DrgbHidController {
    fn drop(&mut self) {
        self.stop_keepalive();
    }
}

impl Controller for DrgbHidController {
    fn port_name(&self) -> String {
        self.path.clone()
    }

    fn model(&self) -> String {
        self.config.name.to_string()
    }

    fn description(&self) -> String {
        "DRGB HID Controller".to_string()
    }

    fn serial_id(&self) -> String {
        self.serial.clone()
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::LedStrip
    }

    fn outputs(&self) -> Vec<OutputPortDefinition> {
        let mut outputs = Vec::new();
        for i in 0..self.config.num_channels {
            outputs.push(OutputPortDefinition {
                id: format!("channel_{}", i),
                name: drgb_output_name(self.config.num_channels, i),
                output_type: SegmentType::Linear,
                leds_count: self.config.leds_per_channel,
                matrix: None,
                capabilities: OutputCapabilities {
                    editable: true,
                    min_total_leds: 0,
                    max_total_leds: self.config.leds_per_channel,
                    allowed_total_leds: None,
                    allowed_segment_types: vec![SegmentType::Linear],
                },
            });
        }
        outputs
    }

    fn update(&mut self, colors: &[Color]) -> Result<(), String> {
        let device = self.device.lock().map_err(|e| e.to_string())?;

        if let Ok(mut last) = self.last_commit.lock() {
            *last = Instant::now();
        }

        // Defensive: if the manager sends a frame shorter than expected, we still send what we have.
        // If it sends longer, we truncate.
        let expected = self.total_leds();
        let colors = if colors.len() == expected {
            colors
        } else if colors.len() > expected {
            &colors[..expected]
        } else {
            colors
        };
        
        let led_total = colors.len();
        let rgb_bytes = self.build_zone_ordered_rgb_bytes(colors);

        match self.config.version {
            4 => {
                // V4: RGBData = 72-byte header + RGB stream
                let mut header = vec![0u8; 72];
                let channels = self.config.num_channels.min(36);
                for i in 0..channels {
                    let led_count = self.config.leds_per_channel;
                    header[i * 2] = ((led_count >> 8) & 0xFF) as u8;
                    header[i * 2 + 1] = (led_count & 0xFF) as u8;
                }

                let mut rgb_data = Vec::with_capacity(72 + rgb_bytes.len());
                rgb_data.extend_from_slice(&header);
                rgb_data.extend_from_slice(&rgb_bytes);

                self.send_packet_v4(&device, &rgb_data, led_total)
            }
            3 => {
                // V3: send 64-byte header (0x60, 0xBB, per-zone LED counts) then RGB payload via SendPacketFS(..., 0x64)
                let mut array_data = [0u8; 64];
                array_data[0] = 0x60;
                array_data[1] = 0xBB;

                let channels = self.config.num_channels.min(31); // up to (zone_idx*2+3) <= 63
                for zone_idx in 0..channels {
                    let lednum = self.config.leds_per_channel;
                    let high = ((lednum >> 8) & 0xFF) as u8;
                    let low = (lednum & 0xFF) as u8;
                    let base = zone_idx * 2 + 2;
                    if base + 1 < 64 {
                        array_data[base] = high;
                        array_data[base + 1] = low;
                    }
                }

                let col_packets = (led_total / DRGB_V3_PACKAGE_SIZE) + usize::from((led_total % DRGB_V3_PACKAGE_SIZE) > 0);
                self.send_packet_fs(&device, &array_data, 1, 0)?;
                self.send_packet_fs(&device, &rgb_bytes, col_packets, 0x64)
            }
            2 => {
                // V2: per-zone packets of 60 bytes payload, each report carries packet index, total packets, zone index, 0xBB
                let leds_per_channel = self.config.leds_per_channel;
                for zone_idx in 0..self.config.num_channels {
                    let start = zone_idx * leds_per_channel;
                    if start >= colors.len() {
                        break;
                    }
                    let end = (start + leds_per_channel).min(colors.len());
                    let zone_bytes = self.build_zone_ordered_rgb_bytes(&colors[start..end]);

                    let lednum = end - start;
                    let num_packets = (lednum / DRGB_V2_PACKAGE_SIZE) + usize::from(!lednum.is_multiple_of(DRGB_V2_PACKAGE_SIZE));
                    for curr_packet in 1..=num_packets {
                        let mut array_data = [0u8; 64];
                        array_data[0] = curr_packet as u8;
                        array_data[1] = num_packets as u8;
                        array_data[2] = zone_idx as u8;
                        array_data[3] = 0xBB;

                        let off = (curr_packet - 1) * 60;
                        for i in 0..60 {
                            array_data[4 + i] = zone_bytes.get(off + i).copied().unwrap_or(0);
                        }

                        self.send_packet_fs(&device, &array_data, 1, 0)?;
                    }
                }
                Ok(())
            }
            1 => {
                // V1: send 64-byte header (0x46, 0xBB, per-zone LED counts) then RGB payload via SendPacketFS(..., 0x47)
                let mut array_data = [0u8; 64];
                array_data[0] = 0x46;
                array_data[1] = 0xBB;

                let channels = self.config.num_channels.min(31);
                for zone_idx in 0..channels {
                    let lednum = self.config.leds_per_channel;
                    let high = ((lednum >> 8) & 0xFF) as u8;
                    let low = (lednum & 0xFF) as u8;
                    let base = zone_idx * 2 + 2;
                    if base + 1 < 64 {
                        array_data[base] = high;
                        array_data[base + 1] = low;
                    }
                }

                let col_packets = (led_total / DRGB_V3_PACKAGE_SIZE) + usize::from((led_total % DRGB_V3_PACKAGE_SIZE) > 0);
                self.send_packet_fs(&device, &array_data, 1, 0)?;
                self.send_packet_fs(&device, &rgb_bytes, col_packets, 0x47)
            }
            v => Err(format!("Unsupported DRGB protocol version: {v}")),
        }
    }

    fn disconnect(&mut self) -> Result<(), String> {
        self.stop_keepalive();
        Ok(())
    }
}

inventory::submit! {
    ControllerMetadata {
        name: "DRGB HID",
        description: "Support for DRGB HID controllers",
        probe: || {
            let mut controllers: Vec<Box<dyn Controller>> = Vec::new();
            
            if let Ok(api) = HidApi::new() {
                for device_info in api.device_list() {
                    let vid = device_info.vendor_id();
                    let pid = device_info.product_id();
                    
                    // Check if it matches any of our known VIDs
                    if vid == DRGBV4_VID || vid == DRGBV3_VID || vid == DRGBV2_VID || vid == YICO_VID {
                        if let Some(config) = get_drgb_config(pid) {
                            if let Ok(device) = device_info.open_device(&api) {
                                let serial = device_info.serial_number().unwrap_or("unknown").to_string();
                                let path = device_info.path().to_string_lossy().to_string();
                                
                                controllers.push(Box::new(DrgbHidController::new(
                                    device,
                                    config,
                                    serial,
                                    path
                                )));
                            }
                        }
                    }
                }
            }
            
            controllers
        },
    }
}
