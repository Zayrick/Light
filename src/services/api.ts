import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, Device, DeviceConfigResponse, EffectInfo } from "../types";
import { logger } from "./logger";

export type CaptureMethod = "dxgi" | "gdi" | "graphics" | "xcap" | "screencapturekit";
export type WindowEffectId = string;

export interface SystemInfo {
  osPlatform: string;
  osVersion: string;
  osBuild: string;
  arch: string;
}

async function invokeWithLog<T>(
  command: string,
  args?: Record<string, unknown>,
  ctx?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (err) {
    logger.error("ipc.invoke_failed", { command, ...ctx }, err);
    throw err;
  }
}

export const api = {
  scanDevices: async (): Promise<Device[]> => {
    return await invokeWithLog<Device[]>("scan_devices");
  },

  getDevices: async (): Promise<Device[]> => {
    return await invokeWithLog<Device[]>("get_devices");
  },

  getDevice: async (port: string): Promise<Device> => {
    return await invokeWithLog<Device>("get_device", { port }, { port });
  },

  getEffects: async (): Promise<EffectInfo[]> => {
    return await invokeWithLog<EffectInfo[]>("get_effects");
  },

  setEffect: async (port: string, effectId: string): Promise<void> => {
    return await invokeWithLog("set_effect", { port, effectId }, { port, effectId });
  },

  updateEffectParams: async (port: string, params: Record<string, unknown>): Promise<void> => {
    return await invokeWithLog("update_effect_params", { port, params }, { port });
  },

  setScopeEffect: async (args: {
    port: string;
    outputId?: string;
    segmentId?: string;
    effectId: string | null;
  }): Promise<void> => {
    const { port, outputId, segmentId, effectId } = args;
    return await invokeWithLog(
      "set_scope_effect",
      { port, outputId, segmentId, effectId },
      { port, outputId, segmentId, effectId }
    );
  },

  updateScopeEffectParams: async (args: {
    port: string;
    outputId?: string;
    segmentId?: string;
    params: Record<string, unknown>;
  }): Promise<void> => {
    const { port, outputId, segmentId, params } = args;
    return await invokeWithLog(
      "update_scope_effect_params",
      { port, outputId, segmentId, params },
      { port, outputId, segmentId }
    );
  },

  setBrightness: async (port: string, brightness: number): Promise<void> => {
    // Legacy device-level brightness.
    return await invokeWithLog("set_brightness", { port, brightness }, { port, brightness });
  },

  setScopeBrightness: async (args: {
    port: string;
    outputId?: string;
    segmentId?: string;
    brightness: number;
  }): Promise<void> => {
    const { port, outputId, segmentId, brightness } = args;
    return await invokeWithLog(
      "set_scope_brightness",
      { port, outputId, segmentId, brightness },
      { port, outputId, segmentId, brightness },
    );
  },

  getCaptureScale: async (): Promise<number> => {
    return await invokeWithLog("get_capture_scale");
  },

  setCaptureScale: async (percent: number): Promise<void> => {
    return await invokeWithLog("set_capture_scale", { percent }, { percent });
  },

  getCaptureFps: async (): Promise<number> => {
    return await invokeWithLog("get_capture_fps");
  },

  setCaptureFps: async (fps: number): Promise<void> => {
    return await invokeWithLog("set_capture_fps", { fps }, { fps });
  },

  getCaptureMethod: async (): Promise<CaptureMethod> => {
    return await invokeWithLog<CaptureMethod>("get_capture_method");
  },

  setCaptureMethod: async (method: CaptureMethod): Promise<void> => {
    return await invokeWithLog("set_capture_method", { method }, { method });
  },

  getWindowEffects: async (): Promise<WindowEffectId[]> => {
    return await invokeWithLog<WindowEffectId[]>("get_window_effects");
  },

  getWindowEffect: async (): Promise<WindowEffectId> => {
    return await invokeWithLog<WindowEffectId>("get_window_effect");
  },

  setWindowEffect: async (effect: WindowEffectId): Promise<void> => {
    return await invokeWithLog("set_window_effect", { effect }, { effect });
  },

  getSystemInfo: async (): Promise<SystemInfo> => {
    return await invokeWithLog<SystemInfo>("get_system_info");
  },

  getMinimizeToTray: async (): Promise<boolean> => {
    return await invokeWithLog<boolean>("get_minimize_to_tray");
  },

  setMinimizeToTray: async (enabled: boolean): Promise<void> => {
    return await invokeWithLog("set_minimize_to_tray", { enabled }, { enabled });
  },

  // --- Persisted configuration

  getAppConfig: async (): Promise<AppConfig> => {
    return await invokeWithLog<AppConfig>("get_app_config");
  },

  setAppConfig: async (config: AppConfig): Promise<AppConfig> => {
    return await invokeWithLog<AppConfig>("set_app_config", { config }, { schemaVersion: config.schemaVersion });
  },

  getDeviceConfig: async (port: string): Promise<DeviceConfigResponse> => {
    return await invokeWithLog<DeviceConfigResponse>("get_device_config", { port }, { port });
  },
};
