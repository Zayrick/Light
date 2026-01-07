export interface CaptureQualityPreset {
  id: string;
  label: string;
  height: number;
  maxPixels: number;
  description: string;
}

export const CAPTURE_QUALITY_PRESETS: CaptureQualityPreset[] = [
  {
    id: "18p",
    label: "18p",
    height: 18,
    maxPixels: 576,
    description: "Micro sample",
  },
  {
    id: "36p",
    label: "36p",
    height: 36,
    maxPixels: 2_304,
    description: "Tiny sample",
  },
  {
    id: "45p",
    label: "45p",
    height: 45,
    maxPixels: 3_600,
    description: "Very low sample",
  },
  {
    id: "90p",
    label: "90p",
    height: 90,
    maxPixels: 14_400,
    description: "LED strip",
  },
  {
    id: "180p",
    label: "180p",
    height: 180,
    maxPixels: 57_600,
    description: "Extreme compression",
  },
  {
    id: "270p",
    label: "270p",
    height: 270,
    maxPixels: 129_600,
    description: "Ultra low",
  },
  {
    id: "360p",
    label: "360p",
    height: 360,
    maxPixels: 230_400,
    description: "Low quality",
  },
  {
    id: "540p",
    label: "540p",
    height: 540,
    maxPixels: 518_400,
    description: "Medium quality",
  },
  {
    id: "720p",
    label: "720p",
    height: 720,
    maxPixels: 921_600,
    description: "High quality",
  },
  {
    id: "1080p",
    label: "1080p",
    height: 1080,
    maxPixels: 2_073_600,
    description: "Top quality",
  },
  {
    id: "original",
    label: "Original",
    height: 0,
    maxPixels: 0,
    description: "No limit (source resolution)",
  },
];

export const DEFAULT_CAPTURE_MAX_PIXELS = 230_400;

const NON_ZERO_PRESETS = CAPTURE_QUALITY_PRESETS.filter((preset) => preset.maxPixels > 0);

export function normalizeCaptureMaxPixels(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_CAPTURE_MAX_PIXELS;
  if (value === 0) return 0;

  const safe = Math.max(1, Math.round(value));
  let closest = NON_ZERO_PRESETS[0];
  let closestDelta = Math.abs(closest.maxPixels - safe);

  for (const preset of NON_ZERO_PRESETS.slice(1)) {
    const delta = Math.abs(preset.maxPixels - safe);
    if (delta < closestDelta || (delta === closestDelta && preset.maxPixels > closest.maxPixels)) {
      closest = preset;
      closestDelta = delta;
    }
  }

  return closest.maxPixels;
}

export function getCaptureQualityIndex(maxPixels: number): number {
  const normalized = normalizeCaptureMaxPixels(maxPixels);
  const index = CAPTURE_QUALITY_PRESETS.findIndex((preset) => preset.maxPixels === normalized);
  if (index >= 0) return index;

  const fallback = CAPTURE_QUALITY_PRESETS.findIndex(
    (preset) => preset.maxPixels === DEFAULT_CAPTURE_MAX_PIXELS,
  );
  return fallback >= 0 ? fallback : 0;
}

export function getCaptureQualityPreset(index: number): CaptureQualityPreset {
  const clamped = Math.min(
    Math.max(0, Math.round(index)),
    CAPTURE_QUALITY_PRESETS.length - 1,
  );
  return CAPTURE_QUALITY_PRESETS[clamped];
}
