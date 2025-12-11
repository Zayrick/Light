import { invoke } from "@tauri-apps/api/core";
import { Device, EffectInfo } from "../types";

export type CaptureMethod = "dxgi" | "gdi" | "graphics" | "xcap";
export type WindowEffectId = string;

export interface SystemInfo {
  osPlatform: string;
  osVersion: string;
  osBuild: string;
  arch: string;
}

export const api = {
  scanDevices: async (): Promise<Device[]> => {
    return await invoke<Device[]>("scan_devices");
  },

  getEffects: async (): Promise<EffectInfo[]> => {
    return await invoke<EffectInfo[]>("get_effects");
  },

  setEffect: async (port: string, effectId: string): Promise<void> => {
    return await invoke("set_effect", { port, effectId });
  },

  updateEffectParams: async (port: string, params: Record<string, unknown>): Promise<void> => {
    return await invoke("update_effect_params", { port, params });
  },

  setBrightness: async (port: string, brightness: number): Promise<void> => {
    return await invoke("set_brightness", { port, brightness });
  },

  getCaptureScale: async (): Promise<number> => {
    return await invoke("get_capture_scale");
  },

  setCaptureScale: async (percent: number): Promise<void> => {
    return await invoke("set_capture_scale", { percent });
  },

  getCaptureFps: async (): Promise<number> => {
    return await invoke("get_capture_fps");
  },

  setCaptureFps: async (fps: number): Promise<void> => {
    return await invoke("set_capture_fps", { fps });
  },

  getCaptureMethod: async (): Promise<CaptureMethod> => {
    return await invoke<CaptureMethod>("get_capture_method");
  },

  setCaptureMethod: async (method: CaptureMethod): Promise<void> => {
    return await invoke("set_capture_method", { method });
  },

  getWindowEffects: async (): Promise<WindowEffectId[]> => {
    return await invoke<WindowEffectId[]>("get_window_effects");
  },

  getWindowEffect: async (): Promise<WindowEffectId> => {
    return await invoke<WindowEffectId>("get_window_effect");
  },

  setWindowEffect: async (effect: WindowEffectId): Promise<void> => {
    return await invoke("set_window_effect", { effect });
  },

  getSystemInfo: async (): Promise<SystemInfo> => {
    return await invoke<SystemInfo>("get_system_info");
  },
};
