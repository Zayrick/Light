from __future__ import annotations

import argparse
import time
from pathlib import Path

from app.core.config import load_device_config
from app.services.virtual_device import VirtualDeviceServer


def main() -> int:
    parser = argparse.ArgumentParser(description="Virtual LED TestDevice")
    parser.add_argument(
        "--config",
        type=str,
        default=None,
        help="Path to device config JSON. When omitted, uses default matrix config.",
    )
    parser.add_argument(
        "--headless",
        action="store_true",
        help="Run UDP service without UI.",
    )
    args = parser.parse_args()

    config_path = Path(args.config) if args.config else None

    if args.headless:
        config = load_device_config(config_path)
        server = VirtualDeviceServer(config)
        server.start()
        try:
            while True:
                time.sleep(0.25)
        except KeyboardInterrupt:
            pass
        finally:
            server.stop()
        return 0

    from app.ui.main_window import run_app

    preset_root = Path(__file__).parent / "presets"
    preset_root.mkdir(parents=True, exist_ok=True)
    return run_app(preset_root, initial_config=config_path)


if __name__ == "__main__":
    raise SystemExit(main())
