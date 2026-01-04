import { ToggleLeft } from "lucide-react";
import { ToggleParam } from "../../../../types";

interface ToggleRendererProps {
  param: ToggleParam;
  value: boolean;
  disabled: boolean;
  onCommit: (value: boolean) => void;
}

/**
 * 纯渲染组件：Toggle 是离散切换，无拖动，直接 commit。
 */
export function ToggleRenderer({
  param,
  value,
  disabled,
  onCommit,
}: ToggleRendererProps) {
  const height = 24;
  const width = (height * 5) / 3; // 40px
  const visualPadding = 2;
  const borderWidth = 1;
  const knobSize = height - visualPadding * 2;
  
  const innerOffset = visualPadding - borderWidth;
  const leftClosed = innerOffset;
  const leftOpen = width - borderWidth * 2 - innerOffset - knobSize;

  const handleClick = () => {
    if (disabled) return;
    onCommit(!value);
  };

  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", height: "50px" }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "6px",
          fontSize: "13px",
          color: "var(--text-secondary)",
        }}
      >
        <ToggleLeft size={16} /> {param.label}
      </div>

      <div
        onClick={handleClick}
        style={{
          width: `${width}px`,
          height: `${height}px`,
          backgroundColor: value ? "var(--accent-color)" : "var(--bg-secondary)",
          border: value ? `1px solid var(--accent-color)` : `1px solid var(--border-subtle)`,
          boxSizing: "border-box",
          borderRadius: "9999px",
          position: "relative",
          cursor: disabled ? "not-allowed" : "pointer",
          opacity: disabled ? 0.5 : 1,
          transition: "background-color 0.3s ease, border-color 0.3s ease",
          flexShrink: 0,
        }}
      >
        <div
          style={{
            position: "absolute",
            top: `${innerOffset}px`,
            left: `${value ? leftOpen : leftClosed}px`,
            width: `${knobSize}px`,
            height: `${knobSize}px`,
            backgroundColor: "white",
            borderRadius: "50%",
            transition: "left 0.3s cubic-bezier(0.4, 0.0, 0.2, 1)",
            boxShadow: "0px 3px 8px rgba(0, 0, 0, 0.15), 0px 3px 1px rgba(0, 0, 0, 0.06)",
          }}
        />
      </div>
    </div>
  );
}

