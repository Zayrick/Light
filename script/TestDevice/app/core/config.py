from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Optional

SCHEMA_VERSION = 1

DEFAULT_DEVICE_NAME = "TestMatrix"
DEFAULT_UDP_PORT = 9999
DEFAULT_PIXEL_SIZE = 6
DEFAULT_MATRIX_WIDTH = 48
DEFAULT_MATRIX_HEIGHT = 27


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


def normalize_output_type(value: Any) -> str:
    if not isinstance(value, str):
        raise ValueError("output_type must be a string")
    v = value.strip()
    if not v:
        raise ValueError("output_type cannot be empty")

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


def leds_count_from_matrix_map(output_id: str, m: MatrixMapConfig) -> int:
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
            raise ValueError(
                f"Output '{output_id}' matrix indices must be non-negative integers or null"
            )
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
        raise ValueError(
            f"Output '{output_id}' matrix indices must cover 0..{leds_count - 1} without gaps"
        )

    return leds_count


def parse_matrix_map(output_id: str, raw: Any) -> MatrixMapConfig:
    if not isinstance(raw, dict):
        raise ValueError(f"Output '{output_id}' matrix must be an object")
    width = int(raw.get("width", 0))
    height = int(raw.get("height", 0))
    raw_map = raw.get("map")
    if not isinstance(raw_map, list):
        raise ValueError(f"Output '{output_id}' matrix.map must be a list")
    m = MatrixMapConfig(width=width, height=height, map=[(None if v is None else int(v)) for v in raw_map])
    _ = leds_count_from_matrix_map(output_id, m)
    return m


def parse_output(raw: Any) -> OutputConfig:
    if not isinstance(raw, dict):
        raise ValueError("Each output must be an object")

    output_id = str(raw.get("id", "")).strip()
    if not output_id:
        raise ValueError("Output id cannot be empty")

    name = str(raw.get("name", "")).strip() or output_id
    output_type = normalize_output_type(raw.get("output_type"))

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

    matrix = parse_matrix_map(output_id, raw.get("matrix"))
    derived = leds_count_from_matrix_map(output_id, matrix)
    hinted = raw.get("leds_count", None)
    if hinted is not None and int(hinted) != derived:
        raise ValueError(
            f"Output '{output_id}' leds_count mismatch: provided={hinted}, derived={derived}"
        )
    return OutputConfig(
        id=output_id,
        name=name,
        output_type=output_type,
        leds_count=derived,
        matrix=matrix,
    )


def device_config_from_dict(raw: Any) -> DeviceConfig:
    if not isinstance(raw, dict):
        raise ValueError("Config root must be an object")

    schema_version = int(raw.get("schema_version", SCHEMA_VERSION))
    device_name = str(raw.get("device_name", DEFAULT_DEVICE_NAME)).strip() or DEFAULT_DEVICE_NAME
    udp_port = int(raw.get("udp_port", DEFAULT_UDP_PORT))
    pixel_size = int(raw.get("pixel_size", DEFAULT_PIXEL_SIZE))

    outputs_raw = raw.get("outputs", [])
    if not isinstance(outputs_raw, list) or not outputs_raw:
        raise ValueError("Config.outputs must be a non-empty list")

    outputs: list[OutputConfig] = []
    ids: set[str] = set()
    for o in outputs_raw:
        out = parse_output(o)
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


def device_config_to_dict(config: DeviceConfig) -> dict[str, Any]:
    outputs = []
    for out in config.outputs:
        item: dict[str, Any] = {
            "id": out.id,
            "name": out.name,
            "output_type": out.output_type,
            "leds_count": out.leds_count,
        }
        if out.output_type == "Single":
            outputs.append(item)
            continue
        if out.output_type == "Linear":
            item["length"] = out.leds_count
            outputs.append(item)
            continue
        if out.output_type == "Matrix" and out.matrix is not None:
            item["matrix"] = {
                "width": out.matrix.width,
                "height": out.matrix.height,
                "map": out.matrix.map,
            }
            outputs.append(item)
            continue
        raise ValueError(f"Unsupported output_type: {out.output_type}")

    return {
        "schema_version": config.schema_version,
        "device_name": config.device_name,
        "udp_port": config.udp_port,
        "pixel_size": config.pixel_size,
        "outputs": outputs,
    }


def default_device_config() -> DeviceConfig:
    w = DEFAULT_MATRIX_WIDTH
    h = DEFAULT_MATRIX_HEIGHT
    m = MatrixMapConfig(width=w, height=h, map=list(range(w * h)))
    out = OutputConfig(id="matrix", name="LED Matrix", output_type="Matrix", leds_count=w * h, matrix=m)
    return DeviceConfig(
        schema_version=SCHEMA_VERSION,
        device_name=DEFAULT_DEVICE_NAME,
        udp_port=DEFAULT_UDP_PORT,
        pixel_size=DEFAULT_PIXEL_SIZE,
        outputs=[out],
    )


def load_device_config(path: Optional[Path]) -> DeviceConfig:
    if path is None:
        return default_device_config()
    raw = json.loads(path.read_text(encoding="utf-8"))
    return device_config_from_dict(raw)


def save_device_config(path: Path, config: DeviceConfig) -> None:
    data = device_config_to_dict(config)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


def build_config_payload(config: DeviceConfig) -> bytes:
    payload = device_config_to_dict(config)
    return json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
