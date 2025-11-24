import { invoke } from "@tauri-apps/api/core";
import { Device, EffectInfo } from "../types";

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
};
