"""
虚拟LED矩阵设备服务端
- 48x27 像素矩阵显示
- mDNS服务发现
- UDP协议接收颜色数据
"""

import socket
import struct
import threading
import pygame
from zeroconf import ServiceInfo, Zeroconf

# 设备配置
MATRIX_WIDTH = 192
MATRIX_HEIGHT = 108
PIXEL_SIZE = 6  # 每个像素在窗口中的显示大小
UDP_PORT = 9999
DEVICE_NAME = "TestMatrix"
SERVICE_TYPE = "_testdevice._udp.local."
SERVICE_NAME = f"{DEVICE_NAME}.{SERVICE_TYPE}"
PROTOCOL_VERSION = 3

# 协议定义
# 命令格式:
# [1字节命令类型] [数据...]
# 命令类型:
#   0x10 - 查询设备信息: 请求 [cmd]，响应 [cmd, version, width_lo, width_hi, height_lo, height_hi, pixel_size_lo, pixel_size_hi, name_len, name_bytes]
#   0x11 - 批量更新像素: [cmd, count_lo, count_hi, (index_lo, index_hi, r, g, b) * count] 线性索引从左上到右下，0-based
#   0x12 - 分片帧数据: [cmd, frame_id, total_fragments, fragment_index, count_lo, count_hi, (index_lo, index_hi, r, g, b) * count]
#          frame_id: 帧序号(0-255循环), total_fragments: 总分片数, fragment_index: 当前分片索引(0-based)
#   0x13 - 帧结束确认: [cmd, frame_id] 可选，用于通知设备一帧已发送完毕

CMD_QUERY_INFO = 0x10
CMD_UPDATE_PIXELS = 0x11
CMD_FRAGMENT_PIXELS = 0x12
CMD_FRAME_END = 0x13

# 分片相关配置
MAX_UDP_PAYLOAD = 1400  # 安全的UDP负载大小，预留MTU余量


class VirtualLEDDevice:
    def __init__(self):
        self.width = MATRIX_WIDTH
        self.height = MATRIX_HEIGHT
        self.pixel_size = PIXEL_SIZE
        self.name = DEVICE_NAME
        
        # 双缓冲机制 - 使用 bytearray 提高性能
        # back_buffer 用于接收数据，front_buffer 用于渲染
        self.buffer_size = self.width * self.height * 3
        self.front_buffer = bytearray(self.buffer_size)
        self.back_buffer = bytearray(self.buffer_size)
        self.buffer_lock = threading.Lock()
        self.need_refresh = True
        
        # 分片帧重组状态
        self.current_frame_id = None
        self.frame_fragments_received = set()
        self.frame_total_fragments = 0
        
        # UDP服务器
        self.udp_socket = None
        self.running = False
        
        # mDNS
        self.zeroconf = None
        self.service_info = None
        
        # Pygame
        self.screen = None
        
    def start(self):
        """启动设备"""
        # 初始化Pygame
        pygame.init()
        window_width = self.width * self.pixel_size
        window_height = self.height * self.pixel_size
        # 使用双缓冲和硬件加速
        self.screen = pygame.display.set_mode(
            (window_width, window_height), 
            pygame.RESIZABLE | pygame.DOUBLEBUF | pygame.HWSURFACE
        )
        pygame.display.set_caption(f"Virtual LED Matrix {self.width}x{self.height}")
        pygame.event.set_blocked(None)  # 阻止所有事件
        pygame.event.set_allowed([pygame.QUIT, pygame.VIDEORESIZE])  # 只允许必要事件
        
        # 启动UDP服务器
        self.running = True
        self.udp_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.udp_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        # 增大接收缓冲区
        self.udp_socket.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 1024 * 1024)
        self.udp_socket.bind(('0.0.0.0', UDP_PORT))
        self.udp_socket.settimeout(0)  # 非阻塞模式
        
        udp_thread = threading.Thread(target=self._udp_listener, daemon=True)
        udp_thread.start()
        
        # 注册mDNS服务
        self._register_mdns()
        
        print(f"虚拟LED设备已启动")
        print(f"矩阵大小: {self.width}x{self.height}")
        print(f"设备名称: {self.name}")
        print(f"UDP端口: {UDP_PORT}")
        print(f"mDNS服务: {SERVICE_NAME}")
        
        # 主循环
        self._main_loop()
        
    def _register_mdns(self):
        """注册mDNS服务"""
        self.zeroconf = Zeroconf()
        
        # 获取本机IP
        hostname = socket.gethostname()
        local_ip = socket.gethostbyname(hostname)
        
        # 服务属性
        properties = {
            'width': str(self.width),
            'height': str(self.height),
            'protocol': 'udp',
            'version': str(PROTOCOL_VERSION),
            'name': self.name
        }
        
        self.service_info = ServiceInfo(
            SERVICE_TYPE,
            SERVICE_NAME,
            addresses=[socket.inet_aton(local_ip)],
            port=UDP_PORT,
            properties=properties,
            server=f"{hostname}.local."
        )
        
        self.zeroconf.register_service(self.service_info)
        print(f"mDNS服务已注册: {local_ip}:{UDP_PORT}")
        
    def _udp_listener(self):
        """UDP数据监听线程 - 高性能非阻塞"""
        import select
        while self.running:
            try:
                # 使用 select 等待数据，超时 10ms
                ready, _, _ = select.select([self.udp_socket], [], [], 0.01)
                if ready:
                    # 批量处理所有待处理的数据包
                    while True:
                        try:
                            data, addr = self.udp_socket.recvfrom(65535)
                            self._process_command(data, addr)
                        except BlockingIOError:
                            break  # 没有更多数据
            except Exception as e:
                if self.running:
                    print(f"UDP错误: {e}")

    def _send_device_info(self, addr):
        """发送设备信息 (协议查询)"""
        if not addr:
            return
        try:
            name_bytes = self.name.encode('utf-8')
            name_len = min(len(name_bytes), 255)
            response = struct.pack(
                '<BBHHHB',
                CMD_QUERY_INFO,
                PROTOCOL_VERSION,
                self.width,
                self.height,
                self.pixel_size,
                name_len
            ) + name_bytes[:name_len]
            self.udp_socket.sendto(response, addr)
        except Exception as e:
            print(f"发送设备信息失败: {e}")
                    
    def _set_pixel_in_buffer(self, index: int, r: int, g: int, b: int):
        """在后台缓冲区设置像素（直接使用线性索引）"""
        idx = index * 3
        if 0 <= idx < self.buffer_size - 2:
            self.back_buffer[idx] = r
            self.back_buffer[idx + 1] = g
            self.back_buffer[idx + 2] = b
    
    def _process_command(self, data, addr=None):
        """处理接收到的命令"""
        if len(data) < 1:
            return
            
        cmd = data[0]
        payload = data[1:]

        # 查询设备信息不需要修改缓冲区
        if cmd == CMD_QUERY_INFO:
            self._send_device_info(addr)
            return

        # 批量更新像素，线性索引 0..(width*height-1)
        if cmd == CMD_UPDATE_PIXELS:
            if len(payload) < 2:
                return
            count = payload[0] | (payload[1] << 8)
            expected_len = 2 + count * 5
            if len(payload) < expected_len:
                count = max(0, (len(payload) - 2) // 5)

            back_buf = self.back_buffer
            buf_size = self.buffer_size
            with self.buffer_lock:
                offset = 2
                for _ in range(count):
                    if offset + 5 > len(payload):
                        break
                    index = payload[offset] | (payload[offset + 1] << 8)
                    idx = index * 3
                    if 0 <= idx < buf_size - 2:
                        back_buf[idx] = payload[offset + 2]
                        back_buf[idx + 1] = payload[offset + 3]
                        back_buf[idx + 2] = payload[offset + 4]
                    offset += 5

                # 写完后自动刷新：交换前后缓冲
                self.front_buffer, self.back_buffer = self.back_buffer, self.front_buffer
                self.need_refresh = True
            return

        # 分片帧数据: [cmd, frame_id, total_fragments, fragment_index, count_lo, count_hi, pixels...]
        if cmd == CMD_FRAGMENT_PIXELS:
            if len(payload) < 5:
                return
            frame_id = payload[0]
            total_fragments = payload[1]
            fragment_index = payload[2]
            count = payload[3] | (payload[4] << 8)
            
            back_buf = self.back_buffer
            buf_size = self.buffer_size
            with self.buffer_lock:
                # 检测新帧：如果 frame_id 变化，重置状态
                if self.current_frame_id != frame_id:
                    self.current_frame_id = frame_id
                    self.frame_fragments_received.clear()
                    self.frame_total_fragments = total_fragments
                
                # 解析像素数据 - 内联优化
                offset = 5
                for _ in range(count):
                    if offset + 5 > len(payload):
                        break
                    index = payload[offset] | (payload[offset + 1] << 8)
                    idx = index * 3
                    if 0 <= idx < buf_size - 2:
                        back_buf[idx] = payload[offset + 2]
                        back_buf[idx + 1] = payload[offset + 3]
                        back_buf[idx + 2] = payload[offset + 4]
                    offset += 5
                
                # 标记此分片已接收
                self.frame_fragments_received.add(fragment_index)
                
                # 检查是否收齐所有分片
                if len(self.frame_fragments_received) >= self.frame_total_fragments:
                    # 交换缓冲区并刷新
                    self.front_buffer, self.back_buffer = self.back_buffer, self.front_buffer
                    self.need_refresh = True
                    # 重置状态等待下一帧
                    self.frame_fragments_received.clear()
            return

        # 帧结束确认（可选，用于强制刷新）
        if cmd == CMD_FRAME_END:
            if len(payload) < 1:
                return
            frame_id = payload[0]
            with self.buffer_lock:
                if self.current_frame_id == frame_id:
                    self.front_buffer, self.back_buffer = self.back_buffer, self.front_buffer
                    self.need_refresh = True
                    self.frame_fragments_received.clear()
            return
                
    def _render(self):
        """渲染像素到屏幕 - 优化版本"""
        with self.buffer_lock:
            # 直接使用 memoryview 避免复制
            pixel_data = memoryview(self.front_buffer)
        
        # 使用 pygame 的 frombuffer 快速创建 surface
        try:
            temp_surface = pygame.image.frombuffer(pixel_data, (self.width, self.height), 'RGB')
            # 获取当前窗口大小并缩放
            window_size = self.screen.get_size()
            # 检查是否需要缩放
            if window_size == (self.width, self.height):
                self.screen.blit(temp_surface, (0, 0))
            else:
                scaled_surface = pygame.transform.scale(temp_surface, window_size)
                self.screen.blit(scaled_surface, (0, 0))
        except Exception as e:
            print(f"渲染错误: {e}")
            
        pygame.display.flip()
        
    def _main_loop(self):
        """主循环 - 非阻塞优化"""
        frame_count = 0
        fps_timer = pygame.time.get_ticks()
        last_render_time = 0
        min_render_interval = 8  # 最小渲染间隔 ~120fps
        
        try:
            while self.running:
                # 非阻塞事件处理
                for event in pygame.event.get():
                    if event.type == pygame.QUIT:
                        self.running = False
                        break
                    elif event.type == pygame.VIDEORESIZE:
                        self.screen = pygame.display.set_mode((event.w, event.h), pygame.RESIZABLE)
                        self.need_refresh = True
                
                current_time = pygame.time.get_ticks()
                
                # 限制渲染频率，但不阻塞
                if self.need_refresh and (current_time - last_render_time) >= min_render_interval:
                    self._render()
                    self.need_refresh = False
                    last_render_time = current_time
                    frame_count += 1
                
                # 每秒更新一次FPS显示
                if current_time - fps_timer >= 1000:
                    fps = frame_count
                    pygame.display.set_caption(f"Virtual LED Matrix {self.width}x{self.height} - {fps} FPS")
                    frame_count = 0
                    fps_timer = current_time
                
                # 短暂休眠避免CPU占用100%，但保持响应性
                pygame.time.delay(1)
                
        finally:
            self._cleanup()
            
    def _cleanup(self):
        """清理资源"""
        print("正在关闭设备...")
        self.running = False
        
        if self.zeroconf and self.service_info:
            self.zeroconf.unregister_service(self.service_info)
            self.zeroconf.close()
            
        if self.udp_socket:
            self.udp_socket.close()
            
        pygame.quit()
        print("设备已关闭")


if __name__ == "__main__":
    device = VirtualLEDDevice()
    device.start()
