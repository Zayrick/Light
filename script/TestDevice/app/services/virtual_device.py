from __future__ import annotations

import socket
import struct
import threading
from typing import Callable, Optional

from ..core.config import DeviceConfig, build_config_payload
from ..core.protocol import (
    CMD_FRAGMENT_PIXELS,
    CMD_QUERY_CONFIG,
    CMD_QUERY_INFO,
    MAX_UDP_PAYLOAD,
    PROTOCOL_VERSION,
)
from ..core.runtime import DeviceRuntime

try:
    from zeroconf import ServiceInfo, Zeroconf  # type: ignore
except Exception:  # pragma: no cover
    ServiceInfo = None  # type: ignore
    Zeroconf = None  # type: ignore

LogFn = Callable[[str], None]


class VirtualDeviceServer:
    def __init__(self, config: DeviceConfig, on_log: Optional[LogFn] = None):
        self.runtime = DeviceRuntime(config)
        self._config_payload = build_config_payload(config)
        self._config_msg_id = 0

        self._on_log = on_log
        self._udp_socket: Optional[socket.socket] = None
        self._thread: Optional[threading.Thread] = None
        self._running = False

        self._zeroconf: Optional[Zeroconf] = None
        self._service_info: Optional[ServiceInfo] = None

    @property
    def running(self) -> bool:
        return self._running

    def start(self) -> None:
        if self._running:
            return

        self._running = True
        self._udp_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        self._udp_socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self._udp_socket.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 1024 * 1024)
        self._udp_socket.bind(("0.0.0.0", self.runtime.udp_port))
        self._udp_socket.settimeout(0)

        self._thread = threading.Thread(target=self._udp_loop, daemon=True)
        self._thread.start()

        self._register_mdns()
        self._log(
            f"Virtual device started: {self.runtime.name} (SN={self.runtime.serial}) UDP={self.runtime.udp_port}"
        )

    def stop(self) -> None:
        if not self._running:
            return
        self._running = False

        if self._udp_socket:
            try:
                self._udp_socket.close()
            except Exception:
                pass
            self._udp_socket = None

        if self._zeroconf and self._service_info:
            try:
                self._zeroconf.unregister_service(self._service_info)
            except Exception:
                pass
            try:
                self._zeroconf.close()
            except Exception:
                pass
            self._service_info = None
            self._zeroconf = None

        if self._thread:
            self._thread.join(timeout=1.0)
            self._thread = None

        self._log("Virtual device stopped")

    def _log(self, message: str) -> None:
        if self._on_log:
            self._on_log(message)

    def _register_mdns(self) -> None:
        if Zeroconf is None or ServiceInfo is None:
            self._log("zeroconf is not available, skipping mDNS")
            return

        hostname = socket.gethostname()
        try:
            local_ip = socket.gethostbyname(hostname)
        except Exception:
            local_ip = "127.0.0.1"

        primary_w, primary_h = self.runtime.primary_dimensions()
        properties = {
            "width": str(primary_w),
            "height": str(primary_h),
            "protocol": "udp",
            "version": str(PROTOCOL_VERSION),
            "name": self.runtime.name,
            "description": self.runtime.description,
            "sn": self.runtime.serial,
            "outputs": str(len(self.runtime.outputs)),
            "leds": str(self.runtime.total_leds),
        }

        service_type = "_testdevice._udp.local."
        # Use SN to keep the service instance name unique across restarts.
        service_name = f"{self.runtime.serial}.{service_type}"

        self._service_info = ServiceInfo(
            service_type,
            service_name,
            addresses=[socket.inet_aton(local_ip)],
            port=self.runtime.udp_port,
            properties=properties,
            server=f"{hostname}.local.",
        )

        self._zeroconf = Zeroconf()
        self._zeroconf.register_service(self._service_info)
        self._log(f"mDNS registered: {local_ip}:{self.runtime.udp_port}")

    def _udp_loop(self) -> None:
        import select

        assert self._udp_socket is not None
        sock = self._udp_socket
        while self._running:
            try:
                ready, _, _ = select.select([sock], [], [], 0.05)
                if not ready:
                    continue
                while True:
                    try:
                        data, addr = sock.recvfrom(65535)
                        self._process_command(data, addr)
                    except BlockingIOError:
                        break
            except Exception:
                if self._running:
                    continue

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

        if cmd == CMD_FRAGMENT_PIXELS:
            if len(payload) < 5:
                return
            frame_id = payload[0]
            total_fragments = payload[1]
            fragment_index = payload[2]
            count = payload[3] | (payload[4] << 8)
            updates = self._parse_updates(payload[5:], count)
            if updates:
                self.runtime.apply_fragment_updates(frame_id, total_fragments, fragment_index, updates)
            return

    def _parse_updates(self, payload: bytes, count_hint: Optional[int] = None) -> list[tuple[int, int, int, int]]:
        if count_hint is None:
            if len(payload) < 2:
                return []
            count = payload[0] | (payload[1] << 8)
            offset = 2
        else:
            count = count_hint
            offset = 0

        expected_len = offset + count * 5
        if len(payload) < expected_len:
            count = max(0, (len(payload) - offset) // 5)

        updates: list[tuple[int, int, int, int]] = []
        for _ in range(count):
            if offset + 5 > len(payload):
                break
            index = payload[offset] | (payload[offset + 1] << 8)
            r = payload[offset + 2]
            g = payload[offset + 3]
            b = payload[offset + 4]
            updates.append((index, r, g, b))
            offset += 5
        return updates

    def _send_device_info(self, addr: tuple[str, int]) -> None:
        if not addr or self._udp_socket is None:
            return
        try:
            primary_w, primary_h = self.runtime.primary_dimensions()
            name_bytes = self.runtime.name.encode("utf-8")
            desc_bytes = self.runtime.description.encode("utf-8")
            sn_bytes = self.runtime.serial.encode("ascii")

            name_len = min(len(name_bytes), 255)
            desc_len = min(len(desc_bytes), 255)
            sn_len = min(len(sn_bytes), 255)

            # Response format (strict, v4):
            # [cmd, version, width_lo, width_hi, height_lo, height_hi, pixel_size_lo, pixel_size_hi,
            #  name_len, name_bytes,
            #  desc_len, desc_bytes,
            #  sn_len, sn_bytes]
            response = (
                struct.pack(
                    "<BBHHH",
                    CMD_QUERY_INFO,
                    PROTOCOL_VERSION,
                    int(primary_w),
                    int(primary_h),
                    int(self.runtime.pixel_size),
                )
                + bytes([name_len])
                + name_bytes[:name_len]
                + bytes([desc_len])
                + desc_bytes[:desc_len]
                + bytes([sn_len])
                + sn_bytes[:sn_len]
            )
            self._udp_socket.sendto(response, addr)
        except Exception:
            return

    def _send_device_config(self, addr: tuple[str, int]) -> None:
        if not addr or self._udp_socket is None:
            return

        payload = self._config_payload
        header_len = 6
        max_chunk = max(1, MAX_UDP_PAYLOAD - header_len)

        total_fragments = (len(payload) + max_chunk - 1) // max_chunk
        if total_fragments > 255:
            self._log("Config payload too large for protocol")
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
            self._udp_socket.sendto(pkt, addr)
