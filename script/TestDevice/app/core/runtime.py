from __future__ import annotations

import threading
from dataclasses import dataclass
from typing import Optional

import secrets

from .config import DeviceConfig, OutputConfig, MatrixMapConfig
from .protocol import PROTOCOL_VERSION

LINEAR_DISPLAY_HEIGHT = 1
OUTPUT_GAP = 2


@dataclass
class OutputRuntime:
    id: str
    name: str
    output_type: str
    leds_count: int
    offset: int
    virtual_width: int
    virtual_height: int
    virtual_to_global: list[Optional[int]]
    render_buffer: bytearray
    top: int
    left: int


class DeviceRuntime:
    def __init__(self, config: DeviceConfig):
        self.config = config
        # Identity is defined by Python (test-only, no backward compatibility).
        self.name = f"TestDevice V{PROTOCOL_VERSION}"
        self.description = "TestDevice by Python mDNS"
        # 16 hex chars, randomized for every process start (UPPERCASE).
        self.serial = secrets.token_hex(8).upper()
        self.udp_port = config.udp_port
        self.pixel_size = config.pixel_size

        self.outputs: list[OutputRuntime] = []
        self.total_leds = 0
        self.canvas_width = 1
        self.canvas_height = 1
        self._build_outputs_runtime()

        self.buffer_size = self.total_leds * 3
        self.front_buffer = bytearray(self.buffer_size)
        self.back_buffer = bytearray(self.buffer_size)
        self.buffer_lock = threading.Lock()
        self.dirty = True

        self.current_frame_id: Optional[int] = None
        self.frame_fragments_received: set[int] = set()
        self.frame_total_fragments = 0

    def _build_outputs_runtime(self) -> None:
        outputs: list[OutputRuntime] = []
        offset = 0
        top = 0
        max_w = 1

        for out in self.config.outputs:
            runtime = self._build_output_runtime(out, offset, top)
            outputs.append(runtime)
            offset += out.leds_count
            top += runtime.virtual_height + OUTPUT_GAP
            max_w = max(max_w, runtime.virtual_width)

        if not outputs:
            raise ValueError("Device must have at least one output")

        self.canvas_width = max_w
        self.canvas_height = max(1, top - OUTPUT_GAP)
        self.total_leds = offset
        self.outputs = outputs

        for o in self.outputs:
            o.left = max(0, (self.canvas_width - o.virtual_width) // 2)

    def _build_output_runtime(self, out: OutputConfig, offset: int, top: int) -> OutputRuntime:
        if out.output_type == "Matrix":
            assert out.matrix is not None
            vw = out.matrix.width
            vh = out.matrix.height
            v2g: list[Optional[int]] = []
            for opt in out.matrix.map:
                if opt is None:
                    v2g.append(None)
                else:
                    v2g.append(offset + int(opt))
            render_buffer = bytearray(vw * vh * 3)
            return OutputRuntime(
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

        if out.output_type == "Linear":
            vw = out.leds_count
            vh = LINEAR_DISPLAY_HEIGHT
            v2g = [offset + i for i in range(vw)]
            render_buffer = bytearray(vw * vh * 3)
            return OutputRuntime(
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

        vw = 1
        vh = 1
        v2g = [offset]
        render_buffer = bytearray(3)
        return OutputRuntime(
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

    def primary_dimensions(self) -> tuple[int, int]:
        for o in self.outputs:
            if o.output_type == "Matrix":
                return (o.virtual_width, o.virtual_height)
        for o in self.outputs:
            if o.output_type == "Linear":
                return (min(o.virtual_width, 65535), 1)
        return (1, 1)

    def mark_dirty(self) -> None:
        with self.buffer_lock:
            self.dirty = True

    def consume_dirty(self) -> bool:
        with self.buffer_lock:
            if self.dirty:
                self.dirty = False
                return True
            return False

    def apply_updates(self, updates: list[tuple[int, int, int, int]]) -> None:
        raise RuntimeError("CMD_UPDATE_PIXELS is not supported in the TestDevice protocol")

    def apply_fragment_updates(
        self,
        frame_id: int,
        total_fragments: int,
        fragment_index: int,
        updates: list[tuple[int, int, int, int]],
    ) -> None:
        with self.buffer_lock:
            if self.current_frame_id != frame_id:
                self.current_frame_id = frame_id
                self.frame_fragments_received.clear()
                self.frame_total_fragments = total_fragments

            back_buf = self.back_buffer
            buf_size = self.buffer_size
            for index, r, g, b in updates:
                idx = index * 3
                if 0 <= idx < buf_size - 2:
                    back_buf[idx] = r
                    back_buf[idx + 1] = g
                    back_buf[idx + 2] = b

            self.frame_fragments_received.add(fragment_index)

            if len(self.frame_fragments_received) >= self.frame_total_fragments:
                self.front_buffer, self.back_buffer = self.back_buffer, self.front_buffer
                self.dirty = True
                self.frame_fragments_received.clear()

    def apply_frame_end(self, frame_id: int) -> None:
        with self.buffer_lock:
            if self.current_frame_id == frame_id:
                self.front_buffer, self.back_buffer = self.back_buffer, self.front_buffer
                self.dirty = True
                self.frame_fragments_received.clear()

    def fill_output_buffers(self) -> None:
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

                for virt_idx, global_idx in enumerate(out.virtual_to_global):
                    dst = virt_idx * 3
                    if global_idx is None:
                        out.render_buffer[dst : dst + 3] = b"\x00\x00\x00"
                        continue
                    src = global_idx * 3
                    out.render_buffer[dst : dst + 3] = front[src : src + 3]
