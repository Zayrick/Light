use crate::interface::controller::{
    Color, Controller, ControllerMetadata, DeviceType, MatrixMap, Zone,
};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

/// LED矩阵UDP控制器
pub struct LedMatrixUdpController {
    device_name: String,
    protocol_version: u8,
    addr: SocketAddr,
    socket: UdpSocket,
    width: usize,
    height: usize,
    led_count: usize,
    zones: Vec<Zone>,
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
        let info = Self::fetch_device_info(&socket, addr)?;

        if info.version != PROTOCOL_VERSION {
            return Err(format!(
                "Protocol version mismatch: device={}, expected={}",
                info.version, PROTOCOL_VERSION
            ));
        }

        let protocol_version = info.version;
        let device_name = info.name.clone();
        let width = info.width as usize;
        let height = info.height as usize;

        let led_count = width * height;

        if led_count > u16::MAX as usize {
            return Err(format!(
                "LED count {} exceeds protocol index limit {}",
                led_count,
                u16::MAX
            ));
        }

        // 创建矩阵映射 - 行优先顺序
        let mut map = Vec::with_capacity(led_count);
        for i in 0..led_count {
            map.push(Some(i));
        }

        let matrix_map = MatrixMap {
            width,
            height,
            map,
        };

        let zones = vec![Zone::matrix("LED Matrix", matrix_map, led_count)];

        // 分片参数与缓冲区预分配
        let max_pixels_per_fragment =
            LedMatrixProtocol::max_pixels_per_fragment(MAX_UDP_PAYLOAD)
                .map_err(|e| format!("Invalid UDP payload setting: {}", e))?;
        // 预分配单个分片的最大空间: cmd(1) + header(5) + pixels * 5
        let frame_buffer = Vec::with_capacity(1 + 5 + max_pixels_per_fragment * 5);

        Ok(Self {
            device_name,
            protocol_version,
            addr,
            socket,
            width,
            height,
            led_count,
            zones,
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
        format!("LED Matrix {}x{}", self.width, self.height)
    }

    fn description(&self) -> String {
        format!(
            "UDP LED Matrix Display ({}x{}) [v{}] - {}",
            self.width, self.height, self.protocol_version, self.device_name
        )
    }

    fn serial_id(&self) -> String {
        self.device_name.clone()
    }

    fn length(&self) -> usize {
        self.led_count
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Virtual
    }

    fn zones(&self) -> Vec<Zone> {
        self.zones.clone()
    }

    fn virtual_layout(&self) -> (usize, usize) {
        (self.width, self.height)
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
            eprintln!("Failed to create mDNS daemon: {}", e);
            return Vec::new();
        }
    };

    // 浏览服务
    let receiver = match mdns.browse(SERVICE_TYPE) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to browse mDNS services: {}", e);
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

                    println!("Discovered LED Matrix: {} at {}:{}", name, device.ip, port);

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

    println!("Scanning for LED Matrix devices via mDNS...");
    let devices = discover_devices(3); // 3秒超时

    for device in devices {
        match LedMatrixUdpController::new(device.clone()) {
            Ok(controller) => {
                println!("Connected to LED Matrix: {}", device.name);
                controllers.push(Box::new(controller));
            }
            Err(e) => {
                eprintln!("Failed to create controller for {}: {}", device.name, e);
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
