import type { CaptureMethod, WindowEffectId } from "../services/api";
import type { SegmentType } from "./device";

export interface ScreenCaptureConfig {
  scalePercent: number;
  fps: number;
  method: CaptureMethod;
}

export interface AppConfig {
  schemaVersion: number;
  windowEffect: WindowEffectId;
  minimizeToTray: boolean;
  screenCapture: ScreenCaptureConfig;
}

// --- Device config persistence (devices/<deviceId>.json)

export interface PersistedModeConfig {
  selected: string | null;
  params: Record<string, Record<string, unknown>>;
}

export interface SegmentDefinition {
  id: string;
  name: string;
  segment_type: SegmentType;
  leds_count: number;
  matrix?: {
    width: number;
    height: number;
    map: Array<number | null>;
  };
}

export interface PersistedDeviceSection {
  /**
   * Segment layout config keyed by output id.
   *
   * Segments are kept as an ordered array because linear outputs derive physical offsets
   * by accumulation.
   */
  layout: Record<
    string,
    {
      segments: SegmentDefinition[];
    }
  >;
}

export interface PersistedSegmentEffectsConfig {
  id: string;
  brightness?: number;
  selected: string | null;
  params: Record<string, Record<string, unknown>>;
}

export interface PersistedOutputEffectsConfig {
  id: string;
  brightness?: number;
  selected: string | null;
  params: Record<string, Record<string, unknown>>;
  segments: PersistedSegmentEffectsConfig[];
}

export interface PersistedEffectsSection {
  // device-level
  selected: string | null;
  params: Record<string, Record<string, unknown>>;
  brightness: number;
  // output / segment-level
  outputs: PersistedOutputEffectsConfig[];
}

export interface PersistedDeviceConfig {
  device: PersistedDeviceSection;
  effects: PersistedEffectsSection;
}

export interface DeviceConfigResponse {
  deviceId: string;
  port: string;
  config: PersistedDeviceConfig | null;
}
