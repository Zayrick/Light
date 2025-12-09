import { ListFilter } from "lucide-react";
import { Select } from "../../../../components/ui/Select";
import { SelectParam } from "../../../../types";

interface SelectRendererProps {
  param: SelectParam;
  value: number;
  disabled: boolean;
  onChange: (value: number) => void;
  onCommit: (value: number) => void;
}

export function SelectRenderer({
  param,
  value,
  disabled,
  onChange,
  onCommit,
}: SelectRendererProps) {
  const hasOptions = param.options.length > 0;

  const handleChange = (val: number) => {
    onChange(val);
    onCommit(val);
  };

  if (!hasOptions) {
    return (
      <div className="select-renderer-empty">
        No options available.
      </div>
    );
  }

  return (
    <Select
      value={value}
      options={param.options}
      onChange={handleChange}
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
