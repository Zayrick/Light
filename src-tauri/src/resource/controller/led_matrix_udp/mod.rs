use crate::interface::controller::{
    Color, Controller, ControllerMetadata, DeviceType, MatrixMap, Zone,
};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod protocol;
use protocol::LedMatrixProtocol;

/// mDNS服务类型
const SERVICE_TYPE: &str = "_ledmatrix._udp.local.";

/// 发现的LED矩阵设备信息
#[derive(Clone, Debug)]
pub struct DiscoveredDevice {
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub width: usize,
    pub height: usize,
}

/// LED矩阵UDP控制器
pub struct LedMatrixUdpController {
    device_name: String,
    addr: SocketAddr,
    socket: UdpSocket,
    width: usize,
    height: usize,
    led_count: usize,
    zones: Vec<Zone>,
    /// 帧缓冲区，用于全量更新
    frame_buffer: Vec<u8>,
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

        let led_count = device.width * device.height;

        // 创建矩阵映射 - 行优先顺序
        let mut map = Vec::with_capacity(led_count);
        for i in 0..led_count {
            map.push(Some(i));
        }

        let matrix_map = MatrixMap {
            width: device.width,
            height: device.height,
            map,
        };

        let zones = vec![Zone::matrix("LED Matrix", matrix_map, led_count)];

        // 预分配帧缓冲区 (1字节命令 + width * height * 3字节颜色)
        let frame_buffer = Vec::with_capacity(1 + led_count * 3);

        Ok(Self {
            device_name: device.name,
            addr,
            socket,
            width: device.width,
            height: device.height,
            led_count,
            zones,
            frame_buffer,
        })
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
            "UDP LED Matrix Display ({}x{}) - {}",
            self.width, self.height, self.device_name
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

        // 使用全量帧更新协议，一次性发送所有数据并自动刷新
        LedMatrixProtocol::encode_full_frame_into(colors, true, &mut self.frame_buffer);
        self.send(&self.frame_buffer)?;

        Ok(())
    }

    fn clear(&mut self) -> Result<(), String> {
        let packet = LedMatrixProtocol::encode_fill_screen(Color { r: 0, g: 0, b: 0 });
        self.send(&packet)?;
        let refresh = LedMatrixProtocol::encode_refresh();
        self.send(&refresh)?;
        Ok(())
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
                    let name = info.get_fullname().to_string();

                    // 获取IP地址
                    let addresses: Vec<_> = info.get_addresses().iter().collect();
                    if addresses.is_empty() {
                        continue;
                    }
                    let ip = addresses[0].to_string();
                    let port = info.get_port();

                    // 从TXT记录获取分辨率
                    let properties = info.get_properties();
                    let width: usize = properties
                        .get_property_val_str("width")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(192);
                    let height: usize = properties
                        .get_property_val_str("height")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(108);

                    let device = DiscoveredDevice {
                        name: name.clone(),
                        ip,
                        port,
                        width,
                        height,
                    };

                    println!(
                        "Discovered LED Matrix: {} at {}:{} ({}x{})",
                        name, device.ip, port, width, height
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

    println!("Scanning for LED Matrix devices via mDNS...");
    let devices = discover_devices(3); // 3秒超时

    for device in devices {
        match LedMatrixUdpController::new(device.clone()) {
            Ok(controller) => {
                println!(
                    "Connected to LED Matrix: {} ({}x{})",
                    device.name, device.width, device.height
                );
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
    probe: probe,
});
