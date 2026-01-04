import { useEffect, useMemo, useRef, useState } from 'react';
import useMeasure from 'react-use-measure';
import { useLedColors } from '../../../hooks/useLedStream';
import { Device, SelectedScope } from '../../../types';
import { computeMultiLayout, processDeviceZones, filterVisibleZones } from '../../../utils/visualizerLayout';

type LedColor = { r: number; g: number; b: number };

/** Helper: draw a single LED cell */
function drawLedCell(
  ctx: CanvasRenderingContext2D,
  x: number, y: number, size: number, radius: number,
  color: LedColor, isDefault: boolean, defaultFill: string
) {
  ctx.fillStyle = isDefault ? defaultFill : `rgb(${color.r},${color.g},${color.b})`;
  roundedRectPath(ctx, x, y, size, size, radius);
  ctx.fill();
}

function roundedRectPath(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number
) {
  const radius = Math.min(r, w / 2, h / 2);
  ctx.beginPath();
  ctx.moveTo(x + radius, y);
  ctx.lineTo(x + w - radius, y);
  ctx.quadraticCurveTo(x + w, y, x + w, y + radius);
  ctx.lineTo(x + w, y + h - radius);
  ctx.quadraticCurveTo(x + w, y + h, x + w - radius, y + h);
  ctx.lineTo(x + radius, y + h);
  ctx.quadraticCurveTo(x, y + h, x, y + h - radius);
  ctx.lineTo(x, y + radius);
  ctx.quadraticCurveTo(x, y, x + radius, y);
  ctx.closePath();
}

interface Props {
  device: Device;
  scope?: SelectedScope;
  onSelectScope?: (scope: SelectedScope) => void;
}

export function DeviceLedVisualizer({ device, scope, onSelectScope }: Props) {
  const [containerRef, bounds] = useMeasure();
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [hoveredKey, setHoveredKey] = useState<string | null>(null);

  // 1. Pre-process outputs into zones
  const { zones: processedZones, totalLeds } = useMemo(
    () => processDeviceZones(device),
    [device]
  );

  // 2. Get colors
  const { colors: physicalColors, isDefault } = useLedColors(device.port, totalLeds);

  // 3. Filter visible zones based on scope
  const visibleZones = useMemo(
    () => filterVisibleZones(processedZones, scope),
    [processedZones, scope]
  );

  // 4. Compute Layout
  const layout = useMemo(
    () => computeMultiLayout(bounds.width, bounds.height, visibleZones, scope),
    [bounds.width, bounds.height, visibleZones, scope]
  );

  const isValidLayout = layout.size > 0 && layout.blocks.length > 0;

  // 4. Render
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !isValidLayout) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const container = canvas.parentElement;
    const styles = container ? getComputedStyle(container) : null;
    const defaultFill = styles?.getPropertyValue('--led-preview-default-fill')?.trim();
    const emptyFill = styles?.getPropertyValue('--led-preview-empty-fill')?.trim();
    const activeBorder =
      styles?.getPropertyValue('--led-preview-active-border')?.trim() ||
      styles?.getPropertyValue('--accent-color')?.trim() ||
      'currentColor';
    const fallbackDefaultFill = 'rgba(128, 128, 128, 0.2)';
    const fallbackEmptyFill = 'rgba(255, 255, 255, 0.06)';
    const effectiveDefaultFill = defaultFill || fallbackDefaultFill;
    const effectiveEmptyFill = emptyFill || fallbackEmptyFill;

    const { width, height, size, gap } = layout;
    const dpr = devicePixelRatio;

    canvas.width = width * dpr;
    canvas.height = height * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);

    const radius = Math.min(2, size / 2);

    const highlightPad = 6;
    const highlightRadius = Math.min(10, Math.max(6, size));

    for (const block of layout.blocks) {
      // Draw Highlight Border if active
      if (block.isActive) {
        ctx.strokeStyle = activeBorder;
        ctx.lineWidth = 1;
        ctx.lineJoin = 'round';
        roundedRectPath(
          ctx,
          block.x - highlightPad,
          block.y - highlightPad,
          block.width + highlightPad * 2,
          block.height + highlightPad * 2,
          highlightRadius
        );
        ctx.stroke();
      }

      for (let row = 0; row < block.rows; row++) {
        for (let col = 0; col < block.cols; col++) {
          const i = row * block.cols + col;
          const x = block.x + col * (size + gap);
          const y = block.y + row * (size + gap);

          if (block.isMatrix) {
            // Matrix: handle null/undefined slots as empty
            const mapVal = block.matrixMap?.[i];
            if (mapVal === null || mapVal === undefined) {
              ctx.fillStyle = effectiveEmptyFill;
              roundedRectPath(ctx, x, y, size, size, radius);
              ctx.fill();
              continue;
            }
            const globalIndex = block.ledStartIndex + mapVal;
            const c = (physicalColors?.[globalIndex] as LedColor | undefined) ?? { r: 0, g: 0, b: 0 };
            drawLedCell(ctx, x, y, size, radius, c, isDefault, effectiveDefaultFill);
          } else {
            // Linear
            if (i >= block.ledCount) continue;
            const globalIndex = block.ledStartIndex + i;
            const c = (physicalColors?.[globalIndex] as LedColor | undefined) ?? { r: 0, g: 0, b: 0 };
            drawLedCell(ctx, x, y, size, radius, c, isDefault, effectiveDefaultFill);
          }
        }
      }
    }

  }, [layout, isValidLayout, physicalColors, isDefault]);

  return (
    <div ref={containerRef} style={{ width: '100%', height: '100%', overflow: 'hidden', position: 'relative' }}>
      <canvas
        ref={canvasRef}
        style={{
          position: 'absolute',
          inset: 0,
          display: 'block',
          width: '100%',
          height: '100%',
        }}
      />

      {/* Interactive overlay: hover label + click-to-jump */}
      {layout.blocks.length > 1 && (
        <div
          style={{
            position: 'absolute',
            inset: 0,
            pointerEvents: 'none',
          }}
        >
          {layout.blocks.map((b) => {
            const key = `${b.outputId}:${b.segmentId ?? ''}`;
            const isHovered = hoveredKey === key;

            // Make hover/click region include the "frame" padding area.
            const hoverPad = 10;
            const left = Math.max(0, b.x - hoverPad);
            const top = Math.max(0, b.y - hoverPad);
            const right = Math.min(layout.width, b.x + b.width + hoverPad);
            const bottom = Math.min(layout.height, b.y + b.height + hoverPad);
            const w = Math.max(0, right - left);
            const h = Math.max(0, bottom - top);

            const handleSelect = () => {
              onSelectScope?.({
                port: device.port,
                outputId: b.outputId,
                segmentId: b.segmentId,
              });
            };

            return (
              <div
                key={key}
                title={b.title}
                style={{
                  position: 'absolute',
                  left,
                  top,
                  width: w,
                  height: h,
                  pointerEvents: 'auto',
                  cursor: onSelectScope ? 'pointer' : 'default',
                  borderRadius: 'var(--radius-m)',
                }}
                role={onSelectScope ? 'button' : undefined}
                tabIndex={onSelectScope ? 0 : -1}
                onMouseEnter={() => setHoveredKey(key)}
                onMouseLeave={() => setHoveredKey(null)}
                onClick={handleSelect}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    handleSelect();
                  }
                }}
              >
                {/* Keep overlay mounted so fade-out can animate */}
                <div
                  style={{
                    position: 'absolute',
                    inset: 0,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    padding: '6px',
                    borderRadius: 'var(--radius-m)',
                    background: 'var(--led-preview-hover-bg)',
                    color: 'var(--led-preview-hover-text)',
                    border: '1px solid var(--border-subtle)',
                    // Stronger blur for a clearer "frosted" effect.
                    backdropFilter: 'blur(18px) saturate(1.25)',
                    WebkitBackdropFilter: 'blur(18px) saturate(1.25)',
                    fontSize: '11px',
                    fontWeight: 600,
                    textAlign: 'center',
                    letterSpacing: '0.2px',
                    pointerEvents: 'none',
                    opacity: isHovered ? 1 : 0,
                    transform: isHovered ? 'scale(1)' : 'scale(0.98)',
                    transition: 'opacity 160ms ease, transform 160ms ease',
                    willChange: 'opacity, transform',
                  }}
                >
                  {b.label}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
