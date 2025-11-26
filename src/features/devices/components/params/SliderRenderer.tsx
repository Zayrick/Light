import { Gauge } from "lucide-react";
import { Slider } from "../../../../components/ui/Slider";
import { SliderParam } from "../../../../types";

interface SliderRendererProps {
  param: SliderParam;
  value: number;
  disabled: boolean;
  onChange: (value: number) => void;
  onCommit: (value: number) => void;
}

export function SliderRenderer({
  param,
  value,
  disabled,
  onChange,
  onCommit,
}: SliderRendererProps) {
  const formatParamValue = (param: SliderParam, value: number) => {
    if (param.step < 1) return value.toFixed(1);
    return Math.round(value).toString();
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          fontSize: "11px",
          color: "var(--text-secondary)",
        }}
      >
        <span style={{ display: "flex", alignItems: "center", gap: "4px" }}>
          <Gauge size={11} /> {param.label}
        </span>
        <span>{formatParamValue(param, value)}</span>
      </div>
      <Slider
        value={value}
        min={param.min}
        max={param.max}
        step={param.step}
        disabled={disabled}
        onChange={onChange}
        onCommit={onCommit}
      />
    </div>
  );
}

