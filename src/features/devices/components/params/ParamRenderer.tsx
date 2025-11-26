import { EffectParam } from "../../../../types";
import { SelectRenderer } from "./SelectRenderer";
import { SliderRenderer } from "./SliderRenderer";
import { ToggleRenderer } from "./ToggleRenderer";

interface ParamRendererProps {
  param: EffectParam;
  modeId: string;
  value: number | boolean;
  disabled: boolean;
  onChange: (value: number | boolean) => void;
  onCommit: (value: number | boolean) => void;
}

/**
 * Dispatcher component that decides which renderer to use based on param.type.
 * This implements the Strategy pattern for UI rendering.
 */
export function ParamRenderer(props: ParamRendererProps) {
  const { param, value, onChange, onCommit } = props;

  switch (param.type) {
    case "slider":
      return (
        <SliderRenderer
          {...props}
          param={param}
          value={value as number}
          onChange={onChange as (v: number) => void}
          onCommit={onCommit as (v: number) => void}
        />
      );
    case "select":
      return (
        <SelectRenderer
          {...props}
          param={param}
          value={value as number}
          onChange={onChange as (v: number) => void}
          onCommit={onCommit as (v: number) => void}
        />
      );
    case "toggle":
      return (
        <ToggleRenderer
          {...props}
          param={param}
          value={value as boolean}
          onChange={onChange as (v: boolean) => void}
          onCommit={onCommit as (v: boolean) => void}
        />
      );
    default:
      console.warn(`No renderer found for param type: ${(param as any).type}`);
      return null;
  }
}
