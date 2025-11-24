export type ZoneType = 'Single' | 'Linear' | 'Matrix';

export interface MatrixMap {
  width: number;
  height: number;
  map: (number | null)[];
}

export interface Zone {
  name: string;
  zone_type: ZoneType;
  start_index: number;
  leds_count: number;
  matrix?: MatrixMap;
}

export interface LedColor {
  r: number;
  g: number;
  b: number;
}

export interface Device {
  port: string;
  model: string;
  description: string;
  id: string;
  length: number;
  zones: Zone[];
  virtual_layout: [number, number]; // Tuple [width, height]
  brightness: number;
}

export interface EffectInfo {
  id: string;
  name: string;
}
