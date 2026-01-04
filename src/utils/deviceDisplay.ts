import type { DeviceType } from "../types";

/**
 * Compact device type label used across the UI (sidebar, home cards, etc.).
 * Keep this mapping as the single source of truth.
 */
export function formatDeviceTypeLabel(deviceType?: DeviceType): string {
  if (!deviceType) return "未知";

  const map: Partial<Record<DeviceType, string>> = {
    Motherboard: "主板",
    Dram: "内存",
    Gpu: "显卡",
    Cooler: "散热",
    LedStrip: "灯带",
    Keyboard: "键盘",
    Mouse: "鼠标",
    MouseMat: "鼠标垫",
    Headset: "耳机",
    HeadsetStand: "耳机架",
    Gamepad: "手柄",
    Light: "灯",
    Speaker: "音箱",
    Virtual: "虚拟设备",
    Storage: "存储",
    Case: "机箱",
    Microphone: "麦克风",
    Accessory: "配件",
    Keypad: "小键盘",
    Laptop: "笔记本",
    Monitor: "显示器",
    Unknown: "未知",
  };

  return map[deviceType] ?? deviceType;
}
