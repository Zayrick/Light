export interface LedColor {
  r: number;
  g: number;
  b: number;
}

export type SegmentType = 'Single' | 'Linear' | 'Matrix';

export type DeviceType =
  | 'Motherboard'
  | 'Dram'
  | 'Gpu'
  | 'Cooler'
  | 'LedStrip'
  | 'Keyboard'
  | 'Mouse'
  | 'MouseMat'
  | 'Headset'
  | 'HeadsetStand'
  | 'Gamepad'
  | 'Light'
  | 'Speaker'
  | 'Virtual'
  | 'Storage'
  | 'Case'
  | 'Microphone'
  | 'Accessory'
  | 'Keypad'
  | 'Laptop'
  | 'Monitor'
  | 'Unknown';

export interface MatrixMap {
  width: number;
  height: number;
  map: (number | null)[];
}

export interface ScopeRef {
  port: string;
  output_id?: string;
  segment_id?: string;
}

export interface ScopeModeState {
  /** Explicit selection at this scope; undefined means inherit parent */
  selected_effect_id?: string;
  /** Resolved selection after inheritance */
  effective_effect_id?: string;
  /** Resolved params for effective effect (from origin scope) */
  effective_params?: Record<string, number | boolean | string>;
  /** Where the effective effect comes from */
  effective_from?: ScopeRef;
}

export interface ScopeBrightnessState {
  /** Stored brightness at this scope (0..=100). */
  value: number;
  /** Resolved brightness after applying inheritance rules. */
  effective_value: number;
  /** Where the effective brightness comes from. */
  effective_from?: ScopeRef;
  /** Whether this scope is currently following its parent brightness. */
  is_following: boolean;
}

export interface OutputCapabilities {
  editable: boolean;
  min_total_leds: number;
  max_total_leds: number;
  allowed_total_leds?: number[];
  allowed_segment_types: SegmentType[];
}

export interface Segment {
  id: string;
  name: string;
  segment_type: SegmentType;
  leds_count: number;
  matrix?: MatrixMap;
  brightness: ScopeBrightnessState;
  mode: ScopeModeState;
}

export interface OutputPort {
  id: string;
  name: string;
  output_type: SegmentType;
  leds_count: number;
  matrix?: MatrixMap;
  capabilities: OutputCapabilities;
  segments: Segment[];
  brightness: ScopeBrightnessState;
  mode: ScopeModeState;
}

export interface Device {
  port: string;
  model: string;
  description: string;
  id: string;
  device_type: DeviceType;
  brightness: ScopeBrightnessState;
  outputs: OutputPort[];
  mode: ScopeModeState;
}

