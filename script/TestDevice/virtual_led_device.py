"""
虚拟 LED 设备服务端 (UDP)

功能：
- 支持多输出口（Matrix / Linear / Single）
- 支持稀疏矩阵（MatrixMap.map 中允许 null）
- mDNS 服务发现
- UDP 协议接收颜色数据（分片帧）
- 通过 UDP 接口查询设备输出配置（JSON，可能分片）

注意：
- 软件侧（Tauri/Rust）应通过接口获取输出/布局信息，而不是直接读取 JSON 文件。
  JSON 只用于启动这个虚拟设备。
"""

from __future__ import annotations

import argparse
import json
import socket
import struct
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Optional

try:
    import pygame  # type: ignore
except Exception:  # pragma: no cover
    pygame = None

try:
    from zeroconf import ServiceInfo, Zeroconf  # type: ignore
except Exception:  # pragma: no cover
    ServiceInfo = None  # type: ignore
    Zeroconf = None  # type: ignore

# -----------------------------------------------------------------------------
# Protocol
# -----------------------------------------------------------------------------

# 命令格式:
# [1字节命令类型] [数据...]
#
# 命令类型:
#   0x10 - 查询设备信息:
#          请求:  [cmd]
#          响应:  [cmd, version, width_lo, width_hi, height_lo, height_hi,
#                 pixel_size_lo, pixel_size_hi, name_len, name_bytes]
#
#   0x11 - 批量更新像素:
#          [cmd, count_lo, count_hi, (index_lo, index_hi, r, g, b) * count]
#          index 为“全设备物理顺序”索引（u16），顺序定义见 Rust 侧 Controller trait 说明。
#
#   0x12 - 分片帧数据:
#          [cmd, frame_id, total_fragments, fragment_index, count_lo, count_hi,
#           (index_lo, index_hi, r, g, b) * count]
#
#   0x13 - 帧结束确认（可选）:
#          [cmd, frame_id]
#
#   0x14 - 查询设备输出配置（JSON，可能分片）:
#          请求: [cmd]
#          响应: [cmd, msg_id, total_fragments, fragment_index, data_len_lo, data_len_hi, data_bytes...]

CMD_QUERY_INFO = 0x10
CMD_UPDATE_PIXELS = 0x11
CMD_FRAGMENT_PIXELS = 0x12
CMD_FRAME_END = 0x13
CMD_QUERY_CONFIG = 0x14

PROTOCOL_VERSION = 4

# 分片相关配置
MAX_UDP_PAYLOAD = 1400  # 安全的UDP负载大小，预留MTU余量


# -----------------------------------------------------------------------------
# Config schema (JSON)
# -----------------------------------------------------------------------------

DEFAULT_DEVICE_NAME = "TestMatrix"
DEFAULT_UDP_PORT = 9999
DEFAULT_PIXEL_SIZE = 6

DEFAULT_MATRIX_WIDTH = 48
DEFAULT_MATRIX_HEIGHT = 27

# Linear outputs look too thin at height=1; we render as multiple rows for visibility.
LINEAR_DISPLAY_HEIGHT = 4

# Gap between outputs in the composite virtual canvas (in virtual pixels).
OUTPUT_GAP = 2


@dataclass(frozen=True)
class MatrixMapConfig:
    width: int
    height: int
    map: list[Optional[int]]


@dataclass(frozen=True)
class OutputConfig:
    id: str
    name: str
    output_type: str  # "Single" | "Linear" | "Matrix"
    leds_count: int
    matrix: Optional[MatrixMapConfig]


@dataclass(frozen=True)
class DeviceConfig:
    schema_version: int
    device_name: str
    udp_port: int
    pixel_size: int
    outputs: list[OutputConfig]


def _normalize_segment_type(value: Any) -> str:
    if not isinstance(value, str):
        raise ValueError("output_type must be a string")
    v = value.strip()
    if not v:
        raise ValueError("output_type cannot be empty")

    # Accept common variants.
    lowered = v.lower()
    if lowered in ("single",):
        return "Single"
    if lowered in ("linear", "strip", "ledstrip"):
        return "Linear"
    if lowered in ("matrix", "grid"):
        return "Matrix"
    if v in ("Single", "Linear", "Matrix"):
        return v
    raise ValueError(f"Unsupported output_type: {value}")


def _leds_count_from_matrix_map(output_id: str, m: MatrixMapConfig) -> int:
    if m.width <= 0 or m.height <= 0:
        raise ValueError(f"Output '{output_id}' has invalid matrix size {m.width}x{m.height}")

    expected_len = m.width * m.height
    if len(m.map) != expected_len:
        raise ValueError(
            f"Output '{output_id}' matrix map length mismatch: expected {expected_len}, got {len(m.map)}"
        )

    max_idx: Optional[int] = None
    for opt in m.map:
        if opt is None:
            continue
        if not isinstance(opt, int) or opt < 0:
            raise ValueError(f"Output '{output_id}' matrix indices must be non-negative integers or null")
        max_idx = opt if max_idx is None else max(max_idx, opt)

    if max_idx is None:
        raise ValueError(f"Output '{output_id}' matrix has no LEDs")

    leds_count = max_idx + 1
    seen = [False] * leds_count
    for opt in m.map:
        if opt is None:
            continue
        if opt >= leds_count:
            raise ValueError(f"Output '{output_id}' matrix index out of range")
        if seen[opt]:
            raise ValueError(f"Output '{output_id}' matrix has duplicate index {opt}")
        seen[opt] = True

    if any(not v for v in seen):
        raise ValueError(f"Output '{output_id}' matrix indices must cover 0..{leds_count - 1} without gaps")

    return leds_count


def _parse_matrix_map(output_id: str, raw: Any) -> MatrixMapConfig:
    if not isinstance(raw, dict):
        raise ValueError(f"Output '{output_id}' matrix must be an object")
    width = int(raw.get("width", 0))
    height = int(raw.get("height", 0))
    raw_map = raw.get("map")
    if not isinstance(raw_map, list):
        raise ValueError(f"Output '{output_id}' matrix.map must be a list")
    m = MatrixMapConfig(width=width, height=height, map=[(None if v is None else int(v)) for v in raw_map])
    _ = _leds_count_from_matrix_map(output_id, m)
    return m


def _parse_output(raw: Any) -> OutputConfig:
    if not isinstance(raw, dict):
        raise ValueError("Each output must be an object")

    output_id = str(raw.get("id", "")).strip()
    if not output_id:
        raise ValueError("Output id cannot be empty")

    name = str(raw.get("name", "")).strip() or output_id
    output_type = _normalize_segment_type(raw.get("output_type"))

    if output_type == "Single":
        leds_count = int(raw.get("leds_count", 1))
        if leds_count != 1:
            raise ValueError(f"Output '{output_id}' is Single but leds_count != 1")
        return OutputConfig(
            id=output_id,
            name=name,
            output_type=output_type,
            leds_count=1,
            matrix=None,
        )

    if output_type == "Linear":
        # Prefer "length" but allow "leds_count".
        length_raw = raw.get("length", None)
        leds_count_raw = raw.get("leds_count", None)
        if length_raw is None and leds_count_raw is None:
            raise ValueError(f"Output '{output_id}' is Linear but missing length")
        length = int(length_raw if length_raw is not None else leds_count_raw)
        if length <= 0:
            raise ValueError(f"Output '{output_id}' has invalid length={length}")
        if length_raw is not None and leds_count_raw is not None and int(length_raw) != int(leds_count_raw):
            raise ValueError(f"Output '{output_id}' has conflicting length and leds_count")
        return OutputConfig(
            id=output_id,
            name=name,
            output_type=output_type,
            leds_count=length,
            matrix=None,
        )

    # Matrix
    matrix = _parse_matrix_map(output_id, raw.get("matrix"))
    derived = _leds_count_from_matrix_map(output_id, matrix)
    hinted = raw.get("leds_count", None)
    if hinted is not None and int(hinted) != derived:
        raise ValueError(f"Output '{output_id}' leds_count mismatch: provided={hinted}, derived={derived}")
    return OutputConfig(
        id=output_id,
        name=name,
        output_type=output_type,
        leds_count=derived,
        matrix=matrix,
    )


def load_device_config(config_path: Optional[str]) -> DeviceConfig:
    if config_path is None:
        # Default: dense 48x27 matrix, row-major.
        w = DEFAULT_MATRIX_WIDTH
        h = DEFAULT_MATRIX_HEIGHT
        m = MatrixMapConfig(width=w, height=h, map=list(range(w * h)))
        out = OutputConfig(id="matrix", name="LED Matrix", output_type="Matrix", leds_count=w * h, matrix=m)
        return DeviceConfig(
            schema_version=1,
            device_name=DEFAULT_DEVICE_NAME,
            udp_port=DEFAULT_UDP_PORT,
            pixel_size=DEFAULT_PIXEL_SIZE,
            outputs=[out],
        )

    path = Path(config_path)
    raw = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        raise ValueError("Config root must be an object")

    schema_version = int(raw.get("schema_version", 1))
    device_name = str(raw.get("device_name", DEFAULT_DEVICE_NAME)).strip() or DEFAULT_DEVICE_NAME
    udp_port = int(raw.get("udp_port", DEFAULT_UDP_PORT))
    pixel_size = int(raw.get("pixel_size", DEFAULT_PIXEL_SIZE))

    outputs_raw = raw.get("outputs", [])
    if not isinstance(outputs_raw, list) or not outputs_raw:
        raise ValueError("Config.outputs must be a non-empty list")

    outputs: list[OutputConfig] = []
    ids: set[str] = set()
    for o in outputs_raw:
        out = _parse_output(o)
        if out.id in ids:
            raise ValueError(f"Duplicate output id: {out.id}")
        ids.add(out.id)
        outputs.append(out)

    return DeviceConfig(
        schema_version=schema_version,
        device_name=device_name,
        udp_port=udp_port,
        pixel_size=pixel_size,
        outputs=outputs,
    )


# -----------------------------------------------------------------------------
# Runtime structures
# -----------------------------------------------------------------------------


@dataclass
class OutputRuntime:
    id: str
    name: str
    output_type: str
    leds_count: int
    offset: int  # global physical offset

    virtual_width: int
    virtual_height: int
    virtual_to_global: list[Optional[int]]  # len = virtual_width * virtual_height

    render_buffer: bytearray  # RGB, len = virtual_width * virtual_height * 3
    top: int  # y offset in composite canvas
    left: int  # x offset in composite canvas


class VirtualLEDDevice:
    def __init__(self, config_path: Optional[str]):
        self.config = load_device_config(config_path)

        self.name = self.config.device_name
        self.udp_port = self.config.udp_port
        self.pixel_size = self.config.pixel_size

        self.outputs: list[OutputRuntime] = []
        self.total_leds: int = 0
        self.canvas_width: int = 1
        self.canvas_height: int = 1
        self._build_outputs_runtime()

        # 双缓冲机制（全设备物理顺序）
        self.buffer_size = self.total_leds * 3
        self.front_buffer = bytearray(self.buffer_size)
        self.back_buffer = bytearray(self.buffer_size)
        self.buffer_lock = threading.Lock()
        self.need_refresh = True

        # 分片帧重组状态
        self.current_frame_id: Optional[int] = None
        self.frame_fragments_received: set[int] = set()
        self.frame_total_fragments = 0

        # Config response
        self._config_msg_id = 0
        self._config_payload = self._build_config_payload()

        # UDP服务器
        self.udp_socket: Optional[socket.socket] = None
        self.running = False

        # mDNS
        self.zeroconf: Optional[Zeroconf] = None
        self.service_info: Optional[ServiceInfo] = None

        # Pygame
        self.screen: Optional[pygame.Surface] = None
        self.canvas_surface: Optional[pygame.Surface] = None

    def _build_outputs_runtime(self) -> None:
        outputs: list[OutputRuntime] = []
        offset = 0
        top = 0
        max_w = 1

        for out in self.config.outputs:
            if out.output_type == "Matrix":
                assert out.matrix is not None
                vw = out.matrix.width
                vh = out.matrix.height
                # virtual_to_global: virtual cell -> global physical index
                v2g: list[Optional[int]] = []
                for opt in out.matrix.map:
                    if opt is None:
                        v2g.append(None)
                    else:
                        v2g.append(offset + int(opt))

                render_buffer = bytearray(vw * vh * 3)
                runtime = OutputRuntime(
                    id=out.id,
                    name=out.name,
                    output_type=out.output_type,
                    leds_count=out.leds_count,
                    offset=offset,
                    virtual_width=vw,
                    virtual_height=vh,
                    virtual_to_global=v2g,
                    render_buffer=render_buffer,
                    top=top,
                    left=0,
                )
                outputs.append(runtime)
                offset += out.leds_count
                top += vh + OUTPUT_GAP
                max_w = max(max_w, vw)
                continue

            if out.output_type == "Linear":
                vw = out.leds_count
                vh = LINEAR_DISPLAY_HEIGHT
                v2g = [offset + i for i in range(vw)]
                render_buffer = bytearray(vw * vh * 3)
                runtime = OutputRuntime(
                    id=out.id,
                    name=out.name,
                    output_type=out.output_type,
                    leds_count=out.leds_count,
                    offset=offset,
                    virtual_width=vw,
                    virtual_height=vh,
                    virtual_to_global=v2g,  # type: ignore[arg-type]
                    render_buffer=render_buffer,
                    top=top,
                    left=0,
                )
                outputs.append(runtime)
                offset += out.leds_count
                top += vh + OUTPUT_GAP
                max_w = max(max_w, vw)
                continue

            # Single
            vw = 1
            vh = 1
            v2g = [offset]
            render_buffer = bytearray(3)
            runtime = OutputRuntime(
                id=out.id,
                name=out.name,
                output_type=out.output_type,
                leds_count=1,
                offset=offset,
                virtual_width=vw,
                virtual_height=vh,
                virtual_to_global=v2g,
                render_buffer=render_buffer,
                top=top,
                left=0,
            )
            outputs.append(runtime)
            offset += 1
            top += vh + OUTPUT_GAP
            max_w = max(max_w, vw)

        if not outputs:
            raise ValueError("Device must have at least one output")

        # Remove trailing gap.
        self.canvas_width = max_w
        self.canvas_height = max(1, top - OUTPUT_GAP)
        self.total_leds = offset
        self.outputs = outputs

        # Center outputs horizontally.
        for o in self.outputs:
            o.left = max(0, (self.canvas_width - o.virtual_width) // 2)

    def _primary_dimensions(self) -> tuple[int, int]:
        # Keep query-info compatible with the legacy single-matrix client.
        for o in self.outputs:
            if o.output_type == "Matrix":
                return (o.virtual_width, o.virtual_height)
        for o in self.outputs:
            if o.output_type == "Linear":
                return (min(o.virtual_width, 65535), 1)
        return (1, 1)

    def _build_config_payload(self) -> bytes:
        # Keep the payload stable and minimal.
        payload: dict[str, Any] = {
            "schema_version": self.config.schema_version,
            "device_name": self.name,
            "outputs": [],
        }

        for out in self.config.outputs:
            item: dict[str, Any] = {
                "id": out.id,
                "name": out.name,
                "output_type": out.output_type,
                "leds_count": out.leds_count,
            }
            if out.output_type == "Linear":
                item["length"] = out.leds_count
            if out.output_type == "Matrix" and out.matrix is not None:
                item["matrix"] = {
                    "width": out.matrix.width,
                    "height": out.matrix.height,
                    "map": out.matrix.map,
                }
            payload["outputs"].append(item)

        return json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8")

    def start(self) -> None:
        """启动设备"""
        if pygame is None:
            raise SystemExit("缺少依赖：pygame。请在 script/TestDevice 下执行: pip install -r requirements.txt")
        pygame.init()

        window_width = max(1, self.canvas_width * self.pixel_size)
        window_height = max(1, self.canvas_height * self.pixel_size)
        self.screen = pygame.display.set_mode(
            (window_width, window_height),
            pygame.RESIZABLE | pygame.DOUBLEBUF | pygame.HWSURFACE,
        )
        pygame.display.set_caption(self._caption(0))
        pygame.event.set_blocked(None)
        pygame.event.set_allowed([pygame.QUIT, pygame.VIDEORESIZE])

        self.canvas_surface = pygame.Surface((self.canvas_width, self.canvas_height))

        # UDP
        self.running = True
        self.udp_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self.udp_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.udp_socket.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 1024 * 1024)
        self.udp_socket.bind(("0.0.0.0", self.udp_port))
        self.udp_socket.settimeout(0)  # non-blocking

        udp_thread = threading.Thread(target=self._udp_listener, daemon=True)
        udp_thread.start()

        # mDNS
        self._register_mdns()

        print("虚拟 LED 设备已启动")
        print(f"设备名称: {self.name}")
        print(f"UDP端口: {self.udp_port}")
        print(f"协议版本: v{PROTOCOL_VERSION}")
        print(f"输出口数量: {len(self.outputs)}  总LED数: {self.total_leds}")
        for o in self.outputs:
            print(f" - {o.id}: {o.name} [{o.output_type}] leds={o.leds_count}")

        self._main_loop()

    def _caption(self, fps: int) -> str:
        return f"Virtual LED Device - {self.name} - {len(self.outputs)} outputs / {self.total_leds} LEDs - {fps} FPS"

    def _register_mdns(self) -> None:
        if Zeroconf is None or ServiceInfo is None:
            raise SystemExit("缺少依赖：zeroconf。请在 script/TestDevice 下执行: pip install -r requirements.txt")
        self.zeroconf = Zeroconf()

        hostname = socket.gethostname()
        local_ip = socket.gethostbyname(hostname)

        primary_w, primary_h = self._primary_dimensions()

        properties = {
            "width": str(primary_w),
            "height": str(primary_h),
            "protocol": "udp",
            "version": str(PROTOCOL_VERSION),
            "name": self.name,
            "outputs": str(len(self.outputs)),
            "leds": str(self.total_leds),
        }

        service_type = "_testdevice._udp.local."
        service_name = f"{self.name}.{service_type}"

        self.service_info = ServiceInfo(
            service_type,
            service_name,
            addresses=[socket.inet_aton(local_ip)],
            port=self.udp_port,
            properties=properties,
            server=f"{hostname}.local.",
        )

        self.zeroconf.register_service(self.service_info)
        print(f"mDNS服务已注册: {local_ip}:{self.udp_port}")

    def _udp_listener(self) -> None:
        import select

        assert self.udp_socket is not None
        while self.running:
            try:
                ready, _, _ = select.select([self.udp_socket], [], [], 0.01)
                if ready:
                    while True:
                        try:
                            data, addr = self.udp_socket.recvfrom(65535)
                            self._process_command(data, addr)
                        except BlockingIOError:
                            break
            except Exception as e:
                if self.running:
                    print(f"UDP错误: {e}")

    def _send_device_info(self, addr: tuple[str, int]) -> None:
        if not addr or self.udp_socket is None:
            return
        try:
            primary_w, primary_h = self._primary_dimensions()
            name_bytes = self.name.encode("utf-8")
            name_len = min(len(name_bytes), 255)
            response = struct.pack(
                "<BBHHHB",
                CMD_QUERY_INFO,
                PROTOCOL_VERSION,
                int(primary_w),
                int(primary_h),
                int(self.pixel_size),
                name_len,
            ) + name_bytes[:name_len]
            self.udp_socket.sendto(response, addr)
        except Exception as e:
            print(f"发送设备信息失败: {e}")

    def _send_device_config(self, addr: tuple[str, int]) -> None:
        if not addr or self.udp_socket is None:
            return

        payload = self._config_payload
        header_len = 6  # cmd + msg_id + total + idx + len(u16)
        max_chunk = max(1, MAX_UDP_PAYLOAD - header_len)

        total_fragments = (len(payload) + max_chunk - 1) // max_chunk
        if total_fragments > 255:
            print("配置数据过大，无法通过协议发送（total_fragments>255）")
            return

        msg_id = self._config_msg_id & 0xFF
        self._config_msg_id = (self._config_msg_id + 1) & 0xFF

        for frag_idx in range(total_fragments):
            start = frag_idx * max_chunk
            end = min(start + max_chunk, len(payload))
            chunk = payload[start:end]
            pkt = (
                bytes([CMD_QUERY_CONFIG, msg_id, total_fragments, frag_idx])
                + len(chunk).to_bytes(2, "little")
                + chunk
            )
            self.udp_socket.sendto(pkt, addr)

    def _process_command(self, data: bytes, addr: tuple[str, int]) -> None:
        if len(data) < 1:
            return

        cmd = data[0]
        payload = data[1:]

        if cmd == CMD_QUERY_INFO:
            self._send_device_info(addr)
            return

        if cmd == CMD_QUERY_CONFIG:
            self._send_device_config(addr)
            return

        # 批量更新像素
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

                self.front_buffer, self.back_buffer = self.back_buffer, self.front_buffer
                self.need_refresh = True
            return

        # 分片帧数据
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
                if self.current_frame_id != frame_id:
                    self.current_frame_id = frame_id
                    self.frame_fragments_received.clear()
                    self.frame_total_fragments = total_fragments

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

                self.frame_fragments_received.add(fragment_index)

                if len(self.frame_fragments_received) >= self.frame_total_fragments:
                    self.front_buffer, self.back_buffer = self.back_buffer, self.front_buffer
                    self.need_refresh = True
                    self.frame_fragments_received.clear()
            return

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

    def _fill_output_buffers(self) -> None:
        # Fill per-output render buffers from the global physical front buffer.
        with self.buffer_lock:
            front = memoryview(self.front_buffer)
            for out in self.outputs:
                if out.output_type == "Linear":
                    w = out.virtual_width
                    h = out.virtual_height
                    for x in range(w):
                        global_idx = out.virtual_to_global[x]
                        if global_idx is None:
                            rgb = b"\x00\x00\x00"
                        else:
                            src = global_idx * 3
                            rgb = front[src : src + 3].tobytes()
                        for y in range(h):
                            dst = (y * w + x) * 3
                            out.render_buffer[dst : dst + 3] = rgb
                    continue

                # Matrix / Single
                for virt_idx, global_idx in enumerate(out.virtual_to_global):
                    dst = virt_idx * 3
                    if global_idx is None:
                        out.render_buffer[dst : dst + 3] = b"\x00\x00\x00"
                        continue
                    src = global_idx * 3
                    out.render_buffer[dst : dst + 3] = front[src : src + 3]

    def _render(self) -> None:
        if self.screen is None or self.canvas_surface is None:
            return

        self._fill_output_buffers()
        self.canvas_surface.fill((0, 0, 0))

        for out in self.outputs:
            try:
                surface = pygame.image.frombuffer(
                    out.render_buffer, (out.virtual_width, out.virtual_height), "RGB"
                )
                self.canvas_surface.blit(surface, (out.left, out.top))
            except Exception as e:
                print(f"渲染输出口失败: {out.id}: {e}")

        window_size = self.screen.get_size()
        if window_size == (self.canvas_width, self.canvas_height):
            self.screen.blit(self.canvas_surface, (0, 0))
        else:
            scaled = pygame.transform.scale(self.canvas_surface, window_size)
            self.screen.blit(scaled, (0, 0))

        pygame.display.flip()

    def _main_loop(self) -> None:
        frame_count = 0
        fps_timer = pygame.time.get_ticks()
        last_render_time = 0
        min_render_interval = 8  # ~120fps

        try:
            while self.running:
                for event in pygame.event.get():
                    if event.type == pygame.QUIT:
                        self.running = False
                        break
                    if event.type == pygame.VIDEORESIZE and self.screen is not None:
                        self.screen = pygame.display.set_mode((event.w, event.h), pygame.RESIZABLE)
                        self.need_refresh = True

                current_time = pygame.time.get_ticks()
                if self.need_refresh and (current_time - last_render_time) >= min_render_interval:
                    self._render()
                    self.need_refresh = False
                    last_render_time = current_time
                    frame_count += 1

                if current_time - fps_timer >= 1000:
                    pygame.display.set_caption(self._caption(frame_count))
                    frame_count = 0
                    fps_timer = current_time

                pygame.time.delay(1)
        finally:
            self._cleanup()

    def _cleanup(self) -> None:
        print("正在关闭设备...")
        self.running = False

        if self.zeroconf and self.service_info:
            self.zeroconf.unregister_service(self.service_info)
            self.zeroconf.close()

        if self.udp_socket:
            self.udp_socket.close()

        pygame.quit()
        print("设备已关闭")


def main() -> int:
    parser = argparse.ArgumentParser(description="Virtual LED Device (UDP)")
    parser.add_argument(
        "--config",
        type=str,
        default=None,
        help="Path to device config JSON (outputs/layout). If omitted, uses a default 48x27 matrix.",
    )
    args = parser.parse_args()

    device = VirtualLEDDevice(config_path=args.config)
    device.start()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
