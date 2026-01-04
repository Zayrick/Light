import { EffectParam, EffectParamValue } from "../../../../types";
import { ColorRenderer } from "./ColorRenderer";
import { SelectRenderer } from "./SelectRenderer";
import { SliderRenderer } from "./SliderRenderer";
import { ToggleRenderer } from "./ToggleRenderer";

interface ParamRendererProps {
  param: EffectParam;
  value: EffectParamValue;
  disabled: boolean;
  onChange: (value: EffectParamValue) => void;
  onCommit: (value: EffectParamValue) => void;
}

/**
 * Dispatcher component that decides which renderer to use based on param.type.
 * This implements the Strategy pattern for UI rendering.
 */
export function ParamRenderer(props: ParamRendererProps) {
  const { param, value, onChange, onCommit, disabled } = props;

  switch (param.type) {
    case "slider":
      return (
        <SliderRenderer
          param={param}
          value={value as number}
          disabled={disabled}
          onChange={onChange as (v: number) => void}
          onCommit={onCommit as (v: number) => void}
        />
      );
    case "select":
      return (
        <SelectRenderer
          param={param}
          value={value as number}
          disabled={disabled}
          onChange={onChange as (v: number) => void}
          onCommit={onCommit as (v: number) => void}
        />
      );
    case "toggle":
      return (
        <ToggleRenderer
          param={param}
          value={value as boolean}
          disabled={disabled}
          onChange={onChange as (v: boolean) => void}
          onCommit={onCommit as (v: boolean) => void}
        />
      );
    case "color":
      return (
        <ColorRenderer
          param={param}
          value={value as string}
          disabled={disabled}
          onChange={onChange as (v: string) => void}
          onCommit={onCommit as (v: string) => void}
        />
      );
    default:
      console.warn(`No renderer found for param type: ${(param as any).type}`);
      return null;
  }
}
