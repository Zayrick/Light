use crate::interface::controller::{
    Color, Controller, ControllerMetadata, DeviceType, MatrixMap, OutputCapabilities,
    OutputPortDefinition, SegmentType,
};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod protocol;
use protocol::{LedMatrixProtocol, PROTOCOL_VERSION, MAX_UDP_PAYLOAD};

/// mDNS服务类型（与虚拟LED矩阵保持一致）
const SERVICE_TYPE: &str = "_testdevice._udp.local.";

/// 发现的LED矩阵设备信息（仅基于mDNS）
#[derive(Clone, Debug)]
pub struct DiscoveredDevice {
    pub name: String,
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
struct DeviceConfigDto {
    #[serde(default)]
    outputs: Vec<OutputPortConfigDto>,
}

#[derive(Debug, Deserialize)]
struct OutputPortConfigDto {
    id: String,
    name: String,
    output_type: SegmentType,
    #[serde(default)]
    length: Option<usize>,
    #[serde(default)]
    matrix: Option<MatrixMap>,
}

/// LED矩阵UDP控制器
pub struct LedMatrixUdpController {
    device_name: String,
    device_description: String,
    serial: String,
    addr: SocketAddr,
    socket: UdpSocket,
    outputs: Vec<OutputPortDefinition>,
    led_count: usize,
    /// 帧缓冲区，用于全量更新
    frame_buffer: Vec<u8>,
    /// 单个分片最多包含的像素数量
    max_pixels_per_fragment: usize,
    /// 当前帧ID（0-255循环）
    frame_id: u8,
}

impl LedMatrixUdpController {
    pub fn new(device: DiscoveredDevice) -> Result<Self, String> {
        let addr: SocketAddr = format!("{}:{}", device.ip, device.port)
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("Failed to bind socket: {}", e))?;
        socket
            .set_nonblocking(false)
            .map_err(|e| format!("Failed to set socket mode: {}", e))?;
        socket
            .set_read_timeout(Some(Duration::from_millis(500)))
            .map_err(|e| format!("Failed to set socket timeout: {}", e))?;

        // 查询设备详细信息（必须成功，否则报错）
        // TestDevice is intentionally not backward-compatible.
        let info = Self::fetch_device_info(&socket, addr)?;

        if info.version != PROTOCOL_VERSION {
            return Err(format!(
                "Unsupported protocol version: device={}, supported={}",
                info.version, PROTOCOL_VERSION
            ));
        }

        let device_name = info.name.clone();
        let device_description = info.description.clone();
        let serial = info.serial.clone();

        let outputs = Self::fetch_device_config(&socket, addr)?;
        if outputs.is_empty() {
            return Err("Device config is empty".to_string());
        }

        let led_count: usize = outputs.iter().map(|o| o.leds_count).sum();
        if led_count == 0 {
            return Err("Device reports zero LEDs".to_string());
        }

        if led_count > u16::MAX as usize {
            return Err(format!(
                "LED count {} exceeds protocol index limit {}",
                led_count,
                u16::MAX
            ));
        }

        // 分片参数与缓冲区预分配
        let max_pixels_per_fragment =
            LedMatrixProtocol::max_pixels_per_fragment(MAX_UDP_PAYLOAD)
                .map_err(|e| format!("Invalid UDP payload setting: {}", e))?;
        // 预分配单个分片的最大空间: cmd(1) + header(5) + pixels * 5
        let frame_buffer = Vec::with_capacity(1 + 5 + max_pixels_per_fragment * 5);

        Ok(Self {
            device_name,
            device_description,
            serial,
            addr,
            socket,
            outputs,
            led_count,
            frame_buffer,
            max_pixels_per_fragment,
            frame_id: 0,
        })
    }

    /// 查询设备信息（必须成功）
    fn fetch_device_info(socket: &UdpSocket, addr: SocketAddr) -> Result<protocol::QueryInfo, String> {
        let payload = LedMatrixProtocol::encode_query_info();
        let mut buf = [0u8; 512];

        for _ in 0..3 {
            socket
                .send_to(&payload, addr)
                .map_err(|e| format!("Failed to send query info: {}", e))?;

            match socket.recv_from(&mut buf) {
                Ok((len, _)) => {
                    if let Some(info) = LedMatrixProtocol::decode_query_response(&buf[..len]) {
                        return Ok(info);
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                    continue;
                }
                Err(e) => return Err(format!("Failed to receive query info: {}", e)),
            }
        }

        Err("No query info response from device".to_string())
    }

    fn fetch_device_config(socket: &UdpSocket, addr: SocketAddr) -> Result<Vec<OutputPortDefinition>, String> {
        let payload = LedMatrixProtocol::encode_query_config();
        let mut buf = [0u8; 65535];

        for _ in 0..3 {
            socket
                .send_to(&payload, addr)
                .map_err(|e| format!("Failed to send query config: {}", e))?;

            let started_at = Instant::now();
            let mut msg_id: Option<u8> = None;
            let mut total_fragments: usize = 0;
            let mut fragments: Vec<Option<Vec<u8>>> = Vec::new();

            loop {
                match socket.recv_from(&mut buf) {
                    Ok((len, _)) => {
                        let Some(fragment) = LedMatrixProtocol::decode_config_fragment(&buf[..len])
                        else {
                            continue;
                        };

                        if msg_id.is_none() {
                            msg_id = Some(fragment.msg_id);
                            total_fragments = fragment.total_fragments as usize;
                            if total_fragments == 0 {
                                break;
                            }
                            fragments = vec![None; total_fragments];
                        }

                        if msg_id != Some(fragment.msg_id) {
                            continue;
                        }
                        if fragment.total_fragments as usize != total_fragments {
                            continue;
                        }

                        let idx = fragment.fragment_index as usize;
                        if idx >= total_fragments {
                            continue;
                        }
                        fragments[idx] = Some(fragment.data.to_vec());

                        if fragments.iter().all(|p| p.is_some()) {
                            let mut bytes = Vec::new();
                            for mut p in fragments.into_iter().flatten() {
                                bytes.append(&mut p);
                            }

                            let cfg: DeviceConfigDto = serde_json::from_slice(&bytes)
                                .map_err(|e| format!("Invalid config JSON: {}", e))?;
                            return Self::build_outputs_from_config(cfg.outputs);
                        }
                    }
                    Err(ref e)
                        if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut =>
                    {
                        break;
                    }
                    Err(e) => return Err(format!("Failed to receive query config: {}", e)),
                }

                // Avoid blocking too long in case of partial packet loss. The socket read timeout is
                // also enforced, but this makes retries more responsive.
                if started_at.elapsed() > Duration::from_millis(800) {
                    break;
                }
            }
        }

        Err("No query config response from device".to_string())
    }

    fn build_outputs_from_config(outputs: Vec<OutputPortConfigDto>) -> Result<Vec<OutputPortDefinition>, String> {
        if outputs.is_empty() {
            return Ok(Vec::new());
        }

        let mut seen_ids: HashSet<String> = HashSet::new();
        let mut defs = Vec::with_capacity(outputs.len());

        for dto in outputs {
            let id = dto.id.trim().to_string();
            if id.is_empty() {
                return Err("Output id cannot be empty".to_string());
            }
            if !seen_ids.insert(id.clone()) {
                return Err(format!("Duplicate output id: {}", id));
            }
            defs.push(Self::output_def_from_dto(dto)?);
        }

        Ok(defs)
    }

    fn output_def_from_dto(dto: OutputPortConfigDto) -> Result<OutputPortDefinition, String> {
        let id = dto.id.trim().to_string();
        let name = if dto.name.trim().is_empty() {
            id.clone()
        } else {
            dto.name
        };

        let output_type = dto.output_type;
        let (leds_count, matrix) = match output_type {
            SegmentType::Single => {
                if dto.matrix.is_some() {
                    return Err(format!("Output '{}' is Single but has matrix data", id));
                }
                (1usize, None)
            }
            SegmentType::Linear => {
                if dto.matrix.is_some() {
                    return Err(format!("Output '{}' is Linear but has matrix data", id));
                }
                let len = dto
                    .length
                    .ok_or_else(|| format!("Output '{}' is Linear but missing length", id))?;
                if len == 0 {
                    return Err(format!("Output '{}' has invalid length=0", id));
                }
                (len, None)
            }
            SegmentType::Matrix => {
                let matrix = dto
                    .matrix
                    .ok_or_else(|| format!("Output '{}' is Matrix but missing matrix data", id))?;
                let derived = Self::leds_count_from_matrix_map(&id, &matrix)?;
                (derived, Some(matrix))
            }
        };

        Ok(OutputPortDefinition {
            id,
            name,
            output_type,
            leds_count,
            matrix,
            capabilities: Self::capabilities_for_output(output_type, leds_count),
        })
    }

    fn leds_count_from_matrix_map(output_id: &str, matrix: &MatrixMap) -> Result<usize, String> {
        let width = matrix.width;
        let height = matrix.height;
        if width == 0 || height == 0 {
            return Err(format!("Output '{}' has invalid matrix size {}x{}", output_id, width, height));
        }
        let expected_len = width
            .checked_mul(height)
            .ok_or_else(|| format!("Output '{}' matrix size overflow", output_id))?;
        if matrix.map.len() != expected_len {
            return Err(format!(
                "Output '{}' matrix map length mismatch: expected {}, got {}",
                output_id,
                expected_len,
                matrix.map.len()
            ));
        }

        let mut max_idx: Option<usize> = None;
        for idx in matrix.map.iter().flatten() {
            max_idx = Some(max_idx.map_or(*idx, |m| m.max(*idx)));
        }

        let Some(max_idx) = max_idx else {
            return Err(format!("Output '{}' matrix has no LEDs", output_id));
        };

        let leds_count = max_idx + 1;
        let mut seen = vec![false; leds_count];

        for opt in &matrix.map {
            let Some(idx) = opt else { continue };
            if *idx >= leds_count {
                return Err(format!("Output '{}' matrix index out of range", output_id));
            }
            if seen[*idx] {
                return Err(format!("Output '{}' matrix has duplicate index {}", output_id, idx));
            }
            seen[*idx] = true;
        }

        if seen.iter().any(|v| !*v) {
            return Err(format!(
                "Output '{}' matrix indices must cover 0..{} without gaps",
                output_id,
                leds_count.saturating_sub(1)
            ));
        }

        Ok(leds_count)
    }

    fn capabilities_for_output(output_type: SegmentType, leds_count: usize) -> OutputCapabilities {
        let allowed_segment_types = match output_type {
            SegmentType::Single => vec![SegmentType::Single],
            SegmentType::Linear => vec![SegmentType::Single, SegmentType::Linear],
            SegmentType::Matrix => vec![SegmentType::Matrix],
        };

        OutputCapabilities {
            editable: output_type == SegmentType::Linear,
            min_total_leds: leds_count,
            max_total_leds: leds_count,
            allowed_total_leds: Some(vec![leds_count]),
            allowed_segment_types,
        }
    }

    /// 发送UDP数据包
    fn send(&self, data: &[u8]) -> Result<(), String> {
        self.socket
            .send_to(data, self.addr)
            .map_err(|e| format!("Failed to send UDP packet: {}", e))?;
        Ok(())
    }
}

impl Controller for LedMatrixUdpController {
    fn port_name(&self) -> String {
        self.addr.to_string()
    }

    fn model(&self) -> String {
        // Device identity is defined by Python.
        self.device_name.clone()
    }

    fn description(&self) -> String {
        self.device_description.clone()
    }

    fn serial_id(&self) -> String {
        self.serial.clone()
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Virtual
    }

    fn outputs(&self) -> Vec<OutputPortDefinition> {
        self.outputs.clone()
    }

    fn update(&mut self, colors: &[Color]) -> Result<(), String> {
        // 验证颜色数组大小
        if colors.len() != self.led_count {
            return Err(format!(
                "Color buffer size mismatch: expected {}, got {}",
                self.led_count,
                colors.len()
            ));
        }

        // 使用分片协议，保证UDP包不会超出安全负载
        let max_pixels = self.max_pixels_per_fragment;
        let total_fragments =
            LedMatrixProtocol::calc_total_fragments(self.led_count, max_pixels)?;
        let frame_id = self.frame_id;
        self.frame_id = self.frame_id.wrapping_add(1);

        for fragment_index in 0..total_fragments {
            let start = fragment_index as usize * max_pixels;
            let end = (start + max_pixels).min(self.led_count);

            LedMatrixProtocol::encode_fragment_into(
                frame_id,
                total_fragments,
                fragment_index,
                start,
                &colors[start..end],
                &mut self.frame_buffer,
            )?;

            self.send(&self.frame_buffer)?;
        }

        Ok(())
    }

    fn clear(&mut self) -> Result<(), String> {
        // 发送全黑帧
        let black = vec![Color { r: 0, g: 0, b: 0 }; self.led_count];
        self.update(&black)
    }

    fn disconnect(&mut self) -> Result<(), String> {
        // 断开前清屏
        self.clear()
    }
}

/// 通过mDNS发现LED矩阵设备
fn discover_devices(timeout_secs: u64) -> Vec<DiscoveredDevice> {
    let devices: Arc<Mutex<HashMap<String, DiscoveredDevice>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // 创建mDNS守护进程
    let mdns = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            log::error!(err:display = e; "Failed to create mDNS daemon");
            return Vec::new();
        }
    };

    // 浏览服务
    let receiver = match mdns.browse(SERVICE_TYPE) {
        Ok(r) => r,
        Err(e) => {
            log::error!(err:display = e; "Failed to browse mDNS services");
            return Vec::new();
        }
    };

    let devices_clone = devices.clone();
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    // 接收服务事件
    while start.elapsed() < timeout {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => match event {
                ServiceEvent::ServiceResolved(info) => {
                    let properties = info.get_properties();

                    let name = properties
                        .get_property_val_str("name")
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| info.get_fullname().to_string());

                    // 获取IP地址
                    let addresses: Vec<_> = info.get_addresses().iter().collect();
                    if addresses.is_empty() {
                        continue;
                    }
                    let ip = addresses[0].to_string();
                    let port = info.get_port();

                    let device = DiscoveredDevice {
                        name: name.clone(),
                        ip,
                        port,
                    };

                    log::info!(
                        name = name.as_str(),
                        ip = device.ip.as_str(),
                        port = port;
                        "Discovered LED Matrix via mDNS"
                    );

                    if let Ok(mut devices) = devices_clone.lock() {
                        devices.insert(name, device);
                    }
                }
                ServiceEvent::ServiceRemoved(_, name) => {
                    if let Ok(mut devices) = devices_clone.lock() {
                        devices.remove(&name);
                    }
                }
                _ => {}
            },
            Err(flume::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        }
    }

    // 停止mDNS守护进程
    let _ = mdns.shutdown();

    // 返回发现的设备
    let result = if let Ok(guard) = devices.lock() {
        guard.values().cloned().collect()
    } else {
        Vec::new()
    };
    result
}

/// 探测函数 - 用于inventory注册
fn probe() -> Vec<Box<dyn Controller>> {
    let mut controllers: Vec<Box<dyn Controller>> = Vec::new();

    log::info!("Scanning for LED Matrix devices via mDNS...");
    let devices = discover_devices(3); // 3秒超时

    for device in devices {
        match LedMatrixUdpController::new(device.clone()) {
            Ok(controller) => {
                log::info!(name = device.name.as_str(); "Connected to LED Matrix");
                controllers.push(Box::new(controller));
            }
            Err(e) => {
                log::warn!(
                    name = device.name.as_str(),
                    err:display = e;
                    "Failed to create LED Matrix controller"
                );
            }
        }
    }

    controllers
}

// 注册控制器到inventory
    inventory::submit!(ControllerMetadata {
    name: "LED Matrix UDP Controller",
    description: "UDP-based LED Matrix Display with mDNS discovery",
    probe,
});
