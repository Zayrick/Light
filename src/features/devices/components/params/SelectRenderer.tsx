import { ListFilter } from "lucide-react";
import { SelectParam } from "../../../../types";
import "./SelectRenderer.css";

interface SelectRendererProps {
  param: SelectParam;
  value: number;
  modeId: string;
  disabled: boolean;
  onChange: (value: number) => void;
  onCommit: (value: number) => void;
}

export function SelectRenderer({
  param,
  value,
  modeId,
  disabled,
  onChange,
  onCommit,
}: SelectRendererProps) {
  const hasOptions = param.options.length > 0;
  const selectId = `${modeId}-${param.key}`;

  const handleChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const val = Number(event.target.value);
    onChange(val);
    onCommit(val);
  };

  return (
    <div className="select-renderer">
      <div className="select-renderer-header">
        <span className="select-renderer-label">
          <ListFilter size={16} /> {param.label}
        </span>
        {hasOptions && (
          <span className="select-renderer-count">
            {param.options.length} option{param.options.length > 1 ? "s" : ""}
          </span>
        )}
      </div>
      {hasOptions ? (
        <select
          id={selectId}
          value={value}
          disabled={disabled}
          onChange={handleChange}
          className="select-renderer-input"
        >
          {param.options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      ) : (
        <div className="select-renderer-empty">
          No options available.
        </div>
      )}
    </div>
  );
}
