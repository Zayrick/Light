from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class PresetInfo:
    name: str
    path: Path
    modified: float


class PresetStore:
    def __init__(self, root: Path):
        self.root = root
        self.root.mkdir(parents=True, exist_ok=True)

    def list_presets(self) -> list[PresetInfo]:
        presets: list[PresetInfo] = []
        for path in sorted(self.root.glob("*.json")):
            presets.append(
                PresetInfo(
                    name=path.stem,
                    path=path,
                    modified=path.stat().st_mtime,
                )
            )
        return presets

    def load_preset(self, path: Path) -> Any:
        return json.loads(path.read_text(encoding="utf-8"))

    def save_preset(self, name: str, data: Any) -> Path:
        safe = self._sanitize_name(name)
        path = self.root / f"{safe}.json"
        path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
        return path

    def delete_preset(self, path: Path) -> None:
        if path.exists():
            path.unlink()

    def _sanitize_name(self, name: str) -> str:
        cleaned = re.sub(r"[^A-Za-z0-9._-]+", "_", name.strip())
        return cleaned or "preset"
