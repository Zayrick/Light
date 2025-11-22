import { useEffect, useMemo, useRef, useState } from "react";
import { LedColor } from "../../../types";

interface LedPreviewPanelProps {
  colors: LedColor[];
  ledCount: number;
}

interface LayoutConfig {
  columns: number;
  cellSize: number;
}

const LED_GAP = 4;
const TARGET_CELL = 12;
const MIN_CELL = 6;
const MAX_CELL = 18;

const ensureColors = (colors: LedColor[], expected: number): LedColor[] => {
  const targetLength = Math.max(expected, colors.length);
  if (targetLength === 0) {
    return [];
  }

  return Array.from({ length: targetLength }, (_, idx) => colors[idx] ?? { r: 0, g: 0, b: 0 });
};

const computeLayout = (width: number, ledCount: number): LayoutConfig => {
  const safeCount = Math.max(ledCount, 1);

  if (width <= 0) {
    return {
      columns: Math.min(safeCount, 8),
      cellSize: TARGET_CELL,
    };
  }

  const approxColumns = Math.max(1, Math.min(safeCount, Math.floor(width / (TARGET_CELL + LED_GAP))));
  const rawSize = (width - (approxColumns - 1) * LED_GAP) / approxColumns;
  const cellSize = Math.min(MAX_CELL, Math.max(MIN_CELL, rawSize));

  return {
    columns: approxColumns,
    cellSize,
  };
};

const isOff = (color: LedColor) => color.r === 0 && color.g === 0 && color.b === 0;

const colorToCss = (color: LedColor) => `rgb(${color.r}, ${color.g}, ${color.b})`;

export function LedPreviewPanel({ colors, ledCount }: LedPreviewPanelProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [panelWidth, setPanelWidth] = useState(0);

  useEffect(() => {
    if (!containerRef.current || typeof ResizeObserver === "undefined") {
      return;
    }

    const observer = new ResizeObserver((entries) => {
      if (!entries.length) return;
      setPanelWidth(entries[0].contentRect.width);
    });

    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, []);

  const normalizedColors = useMemo(() => ensureColors(colors, ledCount), [colors, ledCount]);
  const sliceMiddle = Math.ceil(normalizedColors.length / 2);
  const leftStrip = normalizedColors.slice(0, sliceMiddle);
  const rightStrip = normalizedColors.slice(sliceMiddle);
  const ledsPerSide = Math.max(leftStrip.length, rightStrip.length);
  const layout = useMemo(() => computeLayout(panelWidth, ledsPerSide), [panelWidth, ledsPerSide]);
  const isAnyLit = normalizedColors.some((color) => !isOff(color));
  const statusLabel =
    normalizedColors.length === 0 ? "No LED data" : isAnyLit ? "Mode active" : "Idle";

  const renderStrip = (label: string, stripColors: LedColor[]) => (
    <div className="led-strip-section" key={label}>
      <div className="led-strip-label">{label}</div>
      {stripColors.length === 0 ? (
        <div className="led-preview-empty">Not mapped</div>
      ) : (
        <div
          className="led-strip-grid"
          style={{
            gridTemplateColumns: `repeat(${layout.columns}, ${layout.cellSize}px)`,
            gap: `${LED_GAP}px`,
          }}
        >
          {stripColors.map((color, idx) => {
            const off = isOff(color);
            return (
              <span
                key={`${label}-${idx}`}
                className="led-chip"
                style={{
                  width: layout.cellSize,
                  height: layout.cellSize,
                  backgroundColor: off ? "rgba(255,255,255,0.18)" : colorToCss(color),
                  opacity: off ? 0.55 : 1,
                }}
                title={off ? "Idle" : colorToCss(color)}
              />
            );
          })}
        </div>
      )}
    </div>
  );

  return (
    <div ref={containerRef} className="led-preview-panel">
      <div className="led-preview-header">
        <span className="led-preview-title">Live LEDs</span>
        <span className="led-preview-meta">
          {normalizedColors.length > 0 ? `${normalizedColors.length} leds` : "—"}
        </span>
      </div>
      <div className="led-preview-status">{statusLabel}</div>
      {normalizedColors.length === 0 ? (
        <div className="led-preview-empty">Waiting for LED telemetry...</div>
      ) : (
        <>
          {renderStrip("Left strip", leftStrip)}
          {renderStrip("Right strip", rightStrip)}
        </>
      )}
    </div>
  );
}

