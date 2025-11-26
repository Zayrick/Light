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

export interface EffectInfo {
  id: string;
  name: string;
  description?: string;
  group?: string;
  icon?: string;
  params?: EffectParam[];
}

