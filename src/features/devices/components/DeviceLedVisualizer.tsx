import { useEffect, useMemo, useRef } from 'react';
import useMeasure from 'react-use-measure';
import { useLedColors } from '../../../hooks/useLedStream';
import { Device } from '../../../types';

type LedColor = { r: number; g: number; b: number };

type LayoutData = {
  width: number;
  height: number;
  cols: number;
  rows: number;
  gap: number;
  size: number;
  offsetX: number;
  offsetY: number;
  isMatrix: boolean;
  matrixMap: (number | null)[] | null;
  totalLeds: number;
};

function drawRoundedRect(
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
  ctx.fill();
}

function computeLayout(
  width: number,
  height: number,
  totalLeds: number,
  isMatrix: boolean,
  virtualWidth: number,
  virtualHeight: number,
  matrixMap: (number | null)[] | null
): LayoutData {
  if (width === 0 || height === 0 || totalLeds === 0) {
    return { width, height, cols: 0, rows: 0, gap: 0, size: 0, offsetX: 0, offsetY: 0, isMatrix, matrixMap, totalLeds };
  }

  // Keep the preview stable and cheap to compute.
  // Gap is intentionally simple: matrix tighter, linear slightly wider.
  const gap = isMatrix ? 1 : totalLeds > 200 ? 1 : 2;

  let cols: number;
  let rows: number;

  if (isMatrix) {
    cols = Math.max(1, virtualWidth);
    rows = Math.max(1, virtualHeight);
  } else {
    // Choose a grid whose aspect ratio roughly matches the container.
    // cols ~= sqrt(totalLeds * (width/height))
    const aspect = height > 0 ? width / height : 1;
    cols = Math.max(1, Math.min(totalLeds, Math.ceil(Math.sqrt(totalLeds * aspect))));
    rows = Math.max(1, Math.ceil(totalLeds / cols));
  }

  const wAvail = width - (cols - 1) * gap;
  const hAvail = height - (rows - 1) * gap;
  const size = wAvail > 0 && hAvail > 0 ? Math.max(0, Math.min(wAvail / cols, hAvail / rows)) : 0;

  const gridW = cols * size + (cols - 1) * gap;
  const gridH = rows * size + (rows - 1) * gap;

  return {
    width,
    height,
    cols,
    rows,
    gap,
    size,
    // Align rules:
    // - Matrix: right aligned (matches "panel preview" expectation)
    // - Linear: centered (looks balanced when wrapping)
    offsetX: isMatrix ? Math.max(0, width - gridW) : Math.max(0, (width - gridW) / 2),
    offsetY: Math.max(0, (height - gridH) / 2),
    isMatrix,
    matrixMap,
    totalLeds,
  };
}

interface Props {
  device: Device;
}

export function DeviceLedVisualizer({ device }: Props) {
  const [containerRef, bounds] = useMeasure();
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  const physicalLen = useMemo(() => {
    const sum = device.outputs.reduce((acc, o) => {
      const segSum =
        o.output_type === "Linear" && o.segments.length > 0
          ? o.segments.reduce((sAcc, s) => sAcc + (s.leds_count ?? 0), 0)
          : 0;

      return acc + (segSum > 0 ? segSum : (o.leds_count ?? 0));
    }, 0);
    return Math.max(1, sum);
  }, [device.outputs]);

  const singleOutput = device.outputs.length === 1 ? device.outputs[0] : null;

  const isMatrix =
    singleOutput?.output_type === "Matrix" &&
    !!singleOutput.matrix &&
    singleOutput.matrix.width > 1 &&
    singleOutput.matrix.height > 1;

  const virtualWidth = isMatrix ? singleOutput!.matrix!.width : physicalLen;
  const virtualHeight = isMatrix ? singleOutput!.matrix!.height : 1;
  const virtualLen = Math.max(1, virtualWidth * virtualHeight);
  const matrixMap = isMatrix ? (singleOutput!.matrix!.map ?? null) : null;

  const { colors: physicalColors, isDefault: isDefaultPhysical } = useLedColors(
    device.port,
    physicalLen
  );

  const { colors, isDefault } = useMemo(() => {
    if (!isMatrix || !matrixMap) {
      return { colors: physicalColors, isDefault: isDefaultPhysical };
    }

    // Reconstruct virtual (width*height) color buffer from physical LEDs using the matrix map.
    const out: LedColor[] = Array.from({ length: virtualLen }, () => ({
      r: 0,
      g: 0,
      b: 0,
    }));

    for (let i = 0; i < virtualLen; i++) {
      const phys = matrixMap[i];
      if (phys === null || phys === undefined) continue;
      if (phys >= 0 && phys < physicalColors.length) {
        out[i] = physicalColors[phys] as LedColor;
      }
    }

    return { colors: out, isDefault: isDefaultPhysical };
  }, [isMatrix, matrixMap, physicalColors, isDefaultPhysical, virtualLen]);

  const layout = useMemo(
    () =>
      computeLayout(
        bounds.width,
        bounds.height,
        virtualLen,
        isMatrix,
        virtualWidth,
        virtualHeight,
        matrixMap
      ),
    [bounds.width, bounds.height, virtualLen, isMatrix, virtualWidth, virtualHeight, matrixMap]
  );

  const isValidLayout = layout.cols > 0 && layout.rows > 0 && layout.size > 0;

  // Main-thread rendering (simple + robust).
  // Key behavior: redraw on BOTH color updates and layout updates, so resize never leaves a stretched bitmap.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !isValidLayout) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const container = canvas.parentElement;
    const styles = container ? getComputedStyle(container) : null;
    const defaultFill = styles?.getPropertyValue('--led-preview-default-fill')?.trim();
    const emptyFill = styles?.getPropertyValue('--led-preview-empty-fill')?.trim();
    const fallbackDefaultFill = 'rgba(128, 128, 128, 0.2)';
    const fallbackEmptyFill = 'rgba(255, 255, 255, 0.06)';

    const { width, height, cols, rows, gap, size, offsetX, offsetY } = layout;
    const dpr = devicePixelRatio;

    canvas.width = width * dpr;
    canvas.height = height * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);

    const radius = isMatrix ? Math.min(2, size / 2) : Math.min(4, size / 2);

    for (let row = 0; row < rows; row++) {
      for (let col = 0; col < cols; col++) {
        const i = row * cols + col;
        if (!isMatrix && i >= virtualLen) continue;

        const x = offsetX + col * (size + gap);
        const y = offsetY + row * (size + gap);

        if (isMatrix && matrixMap?.[i] === null) {
          ctx.fillStyle = emptyFill || fallbackEmptyFill;
          drawRoundedRect(ctx, x, y, size, size, radius);
          continue;
        }

        const c = (colors?.[i] as LedColor | undefined) ?? { r: 0, g: 0, b: 0 };
        ctx.fillStyle = isDefault ? (defaultFill || fallbackDefaultFill) : `rgb(${c.r},${c.g},${c.b})`;
        drawRoundedRect(ctx, x, y, size, size, radius);
      }
    }
  }, [layout, isValidLayout, colors, isDefault, isMatrix, virtualLen, matrixMap]);

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
    </div>
  );
}
