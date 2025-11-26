import FormControl from "@mui/material/FormControl";
import Select, { SelectChangeEvent } from "@mui/material/Select";
import MenuItem from "@mui/material/MenuItem";
import { ListFilter } from "lucide-react";
import { SelectParam } from "../../../../types";

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
  const selectLabelId = `${modeId}-${param.key}-label`;

  const handleChange = (event: SelectChangeEvent<string>) => {
    const val = Number(event.target.value);
    onChange(val);
    onCommit(val);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          fontSize: "11px",
          color: "var(--text-secondary)",
          alignItems: "center",
        }}
      >
        <span style={{ display: "flex", alignItems: "center", gap: "4px" }}>
          <ListFilter size={11} /> {param.label}
        </span>
        {hasOptions && (
          <span style={{ opacity: 0.7 }}>
            {param.options.length} option{param.options.length > 1 ? "s" : ""}
          </span>
        )}
      </div>
      {hasOptions ? (
        <FormControl fullWidth size="small" variant="outlined">
          <Select
            labelId={selectLabelId}
            value={String(value)}
            disabled={disabled}
            onChange={handleChange}
            MenuProps={{
              PaperProps: {
                sx: {
                  maxHeight: 280,
                  backgroundColor: "var(--bg-card)",
                  backdropFilter: "blur(20px)",
                  color: "var(--text-primary)",
                  borderRadius: "var(--radius-m)",
                  border: "1px solid var(--border-subtle)",
                  "& .MuiMenuItem-root": {
                    "&.Mui-selected": {
                      backgroundColor: "var(--accent-color)",
                      color: "var(--accent-text)",
                      "&:hover": {
                        backgroundColor: "var(--accent-hover)",
                      },
                    },
                    "&:hover": {
                      backgroundColor: "var(--bg-card-hover)",
                    },
                    fontSize: "13px",
                    minHeight: "32px",
                  },
                },
              },
            }}
            sx={{
              color: "var(--text-primary)",
              borderRadius: "var(--radius-m)",
              fontSize: "13px",
              height: "32px",
              ".MuiOutlinedInput-notchedOutline": {
                borderColor: "var(--border-subtle)",
              },
              "&:hover .MuiOutlinedInput-notchedOutline": {
                borderColor: "var(--text-secondary)",
              },
              "&.Mui-focused .MuiOutlinedInput-notchedOutline": {
                borderColor: "var(--accent-color)",
              },
              ".MuiSvgIcon-root": {
                color: "var(--text-secondary)",
              },
              ".MuiSelect-select": {
                display: "flex",
                alignItems: "center",
                paddingTop: "4px",
                paddingBottom: "4px",
              },
            }}
          >
            {param.options.map((option) => (
              <MenuItem key={option.value} value={String(option.value)}>
                {option.label}
              </MenuItem>
            ))}
          </Select>
        </FormControl>
      ) : (
        <div style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
          No options available.
        </div>
      )}
    </div>
  );
}

