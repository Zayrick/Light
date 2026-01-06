import type { AppConfig, ScreenCaptureConfig } from "../types";
import type { CaptureMethod, WindowEffectId } from "./api";
import { api } from "./api";
import { normalizeCaptureMaxPixels } from "../utils/captureQuality";

let cachedAppConfig: AppConfig | null = null;
let loadingPromise: Promise<AppConfig> | null = null;

function clamp(n: number, min: number, max: number) {
  return Math.min(max, Math.max(min, n));
}

function normalizeAppConfig(cfg: AppConfig): AppConfig {
  return {
    ...cfg,
    screenCapture: {
      ...cfg.screenCapture,
      maxPixels: normalizeCaptureMaxPixels(cfg.screenCapture.maxPixels),
      fps: clamp(cfg.screenCapture.fps, 1, 60),
    },
  };
}

export const configManager = {
  getAppConfig: async (): Promise<AppConfig> => {
    if (cachedAppConfig) return cachedAppConfig;
    if (loadingPromise) return loadingPromise;

    loadingPromise = api
      .getAppConfig()
      .then((cfg) => {
        cachedAppConfig = normalizeAppConfig(cfg);
        return cachedAppConfig;
      })
      .finally(() => {
        loadingPromise = null;
      });

    return loadingPromise;
  },

  setAppConfig: async (next: AppConfig): Promise<AppConfig> => {
    const normalized = normalizeAppConfig(next);
    const saved = await api.setAppConfig(normalized);
    cachedAppConfig = normalizeAppConfig(saved);
    return cachedAppConfig;
  },

  updateAppConfig: async (patch: Partial<AppConfig>): Promise<AppConfig> => {
    const current = await configManager.getAppConfig();

    // Manual merge (avoid bringing a generic deep-merge dependency).
    const merged: AppConfig = {
      ...current,
      ...patch,
      screenCapture: {
        ...current.screenCapture,
        ...(patch.screenCapture ?? {}),
      } as ScreenCaptureConfig,
    };

    return await configManager.setAppConfig(merged);
  },

  setMinimizeToTray: async (enabled: boolean): Promise<AppConfig> => {
    return await configManager.updateAppConfig({ minimizeToTray: enabled });
  },

  setWindowEffect: async (effect: WindowEffectId): Promise<AppConfig> => {
    return await configManager.updateAppConfig({ windowEffect: effect });
  },

  setCaptureMethod: async (method: CaptureMethod): Promise<AppConfig> => {
    return await configManager.updateAppConfig({ screenCapture: { method } as ScreenCaptureConfig });
  },

  setCaptureMaxPixels: async (maxPixels: number): Promise<AppConfig> => {
    return await configManager.updateAppConfig({
      screenCapture: { maxPixels } as ScreenCaptureConfig,
    });
  },

  setCaptureFps: async (fps: number): Promise<AppConfig> => {
    return await configManager.updateAppConfig({ screenCapture: { fps } as ScreenCaptureConfig });
  },

  // Useful in dev / debug pages.
  getDeviceConfig: async (port: string) => {
    return await api.getDeviceConfig(port);
  },
};
