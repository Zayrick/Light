import { ListFilter } from "lucide-react";
import { Select } from "../../../../components/ui/Select";
import { SelectParam } from "../../../../types";

interface SelectRendererProps {
  param: SelectParam;
  value: number;
  disabled: boolean;
  onCommit: (value: number) => void;
}

/**
 * 纯渲染组件：Select 是离散选择，无拖动，直接 commit。
 */
export function SelectRenderer({
  param,
  value,
  disabled,
  onCommit,
}: SelectRendererProps) {
  if (param.options.length === 0) {
    return <div className="select-renderer-empty">No options available.</div>;
  }

  return (
    <Select
      value={value}
      options={param.options}
      onChange={onCommit}
      disabled={disabled}
      label={
        <span style={{ display: "flex", alignItems: "center", gap: "6px" }}>
          <ListFilter size={16} /> {param.label}
        </span>
      }
      valueText={`${param.options.length} option${param.options.length > 1 ? "s" : ""}`}
    />
  );
}
