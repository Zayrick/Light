import { EffectParam } from "../../../../types";
import { SelectRenderer } from "./SelectRenderer";
import { SliderRenderer } from "./SliderRenderer";

interface ParamRendererProps {
  param: EffectParam;
  modeId: string;
  value: number;
  disabled: boolean;
  onChange: (value: number) => void;
  onCommit: (value: number) => void;
}

/**
 * Dispatcher component that decides which renderer to use based on param.type.
 * This implements the Strategy pattern for UI rendering.
 */
export function ParamRenderer(props: ParamRendererProps) {
  const { param } = props;

  switch (param.type) {
    case "slider":
      return <SliderRenderer {...props} param={param} />;
    case "select":
      return <SelectRenderer {...props} param={param} />;
    default:
      console.warn(`No renderer found for param type: ${(param as any).type}`);
      return null;
  }
}
