import { Palette } from "lucide-react";
import { ColorPicker } from "../../../../components/ui/ColorPicker";
import { ColorParam } from "../../../../types";

interface ColorRendererProps {
  param: ColorParam;
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onCommit: (value: string) => void;
}

export function ColorRenderer({
  param,
  value,
  disabled,
  onChange,
  onCommit,
}: ColorRendererProps) {
  return (
    <ColorPicker
      value={value}
      disabled={disabled}
      label={
        <span style={{ display: "flex", alignItems: "center", gap: "6px" }}>
          <Palette size={16} /> {param.label}
        </span>
      }
      onChange={onChange}
      onCommit={onCommit}
    />
  );
}
