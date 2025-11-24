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
  current_effect_id?: string;
}

export interface EffectInfo {
  id: string;
  name: string;
  description?: string;
  group?: string;
  params?: EffectParam[];
}

export type ParamDependencyBehavior = 'hide' | 'disable';

export interface ParamDependency {
  key: string;
  equals?: number;
  notEquals?: number;
  behavior?: ParamDependencyBehavior;
}

interface EffectParamBase {
  key: string;
  label: string;
  default: number;
  dependency?: ParamDependency;
}

export interface SliderParam extends EffectParamBase {
  type: 'slider';
  min: number;
  max: number;
  step: number;
}

export interface SelectOption {
  label: string;
  value: number;
}

export interface SelectParam extends EffectParamBase {
  type: 'select';
  options: SelectOption[];
}

export type EffectParam = SliderParam | SelectParam;
