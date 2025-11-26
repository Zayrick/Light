export type ParamDependencyBehavior = 'hide' | 'disable';

export interface ParamDependency {
  key?: string;
  equals?: number;
  notEquals?: number;
  behavior?: ParamDependencyBehavior;
}

interface EffectParamBase {
  key: string;
  label: string;
  default: number | boolean;
  dependency?: ParamDependency;
}

export interface SliderParam extends EffectParamBase {
  type: 'slider';
  default: number;
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
  default: number;
  options: SelectOption[];
}

export interface ToggleParam extends EffectParamBase {
  type: 'toggle';
  default: boolean;
}

export type EffectParam = SliderParam | SelectParam | ToggleParam;

export interface EffectInfo {
  id: string;
  name: string;
  description?: string;
  group?: string;
  icon?: string;
  params?: EffectParam[];
}
