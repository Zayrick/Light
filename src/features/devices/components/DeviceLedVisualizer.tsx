import { useEffect, useMemo, useRef } from 'react';
import useMeasure from 'react-use-measure';
import { useLedColors } from '../../../hooks/useLedStream';
import { Device } from '../../../types';
import type { LayoutData, LedColor } from './ledPreviewTypes';

// Static feature detection (never changes at runtime)
const SUPPORTS_OFFSCREEN =
  typeof OffscreenCanvas !== 'undefined' &&
  typeof HTMLCanvasElement !== 'undefined' &&
  'transferControlToOffscreen' in HTMLCanvasElement.prototype;

const DEFAULT_FILL = 'rgba(128, 128, 128, 0.2)';
const EMPTY_CELL_FILL = 'rgba(255, 255, 255, 0.06)';

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

  // Dynamic gap
  let gap = 2;
  if (isMatrix) {
    const maxDim = Math.max(virtualWidth, virtualHeight);
    if (maxDim > 40) gap = 0;
    else if (maxDim > 20) gap = 1;
  } else if (totalLeds > 100) {
    gap = 1;
  }

  let cols: number, rows: number, size: number;

  if (isMatrix) {
    cols = virtualWidth;
    rows = virtualHeight;
    const wAvail = width - (cols - 1) * gap;
    const hAvail = height - (rows - 1) * gap;
    size = wAvail > 0 && hAvail > 0 ? Math.max(0, Math.min(wAvail / cols, hAvail / rows)) : 0;
  } else {
    // Smart wrapping for linear strips
    let best = { size: 0, cols: 1, rows: 1 };
    for (let c = 1; c <= totalLeds; c++) {
      const r = Math.ceil(totalLeds / c);
      const wAvail = width - (c - 1) * gap;
      const hAvail = height - (r - 1) * gap;
      if (wAvail <= 0 || hAvail <= 0) continue;
      const s = Math.min(wAvail / c, hAvail / r);
      if (s > best.size) best = { size: s, cols: c, rows: r };
    }
    cols = best.cols;
    rows = best.rows;
    size = Math.max(0, best.size);
  }

  const gridW = cols * size + (cols - 1) * gap;
  const gridH = rows * size + (rows - 1) * gap;

  return {
    width,
    height,
    cols,
    rows,
    gap,
    size,
    offsetX: Math.max(0, width - gridW),
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
  const workerRef = useRef<Worker | null>(null);
  const workerReady = useRef(false);

  const virtualWidth = device.virtual_layout?.[0] ?? device.length;
  const virtualHeight = device.virtual_layout?.[1] ?? 1;
  const totalLeds = virtualWidth * virtualHeight;
  const isMatrix = virtualHeight > 1;
  const matrixMap = device.zones.find((z) => z.zone_type === 'Matrix')?.matrix?.map ?? null;

  const { colors, isDefault } = useLedColors(device.port, totalLeds);

  const layout = useMemo(
    () => computeLayout(bounds.width, bounds.height, totalLeds, isMatrix, virtualWidth, virtualHeight, matrixMap),
    [bounds.width, bounds.height, totalLeds, isMatrix, virtualWidth, virtualHeight, matrixMap]
  );

  const isValidLayout = layout.cols > 0 && layout.rows > 0 && layout.size > 0;

  // Worker path: init + layout updates
  useEffect(() => {
    if (!SUPPORTS_OFFSCREEN) return;
    const canvas = canvasRef.current;
    if (!canvas || !isValidLayout) return;

    if (!workerRef.current) {
      workerRef.current = new Worker(new URL('./ledPreview.worker.ts', import.meta.url), { type: 'module' });
      workerReady.current = false;
    }

    if (!workerReady.current) {
      const offscreen = canvas.transferControlToOffscreen();
      workerRef.current.postMessage({ type: 'init', canvas: offscreen, dpr: devicePixelRatio }, [offscreen]);
      workerReady.current = true;
    }

    workerRef.current.postMessage({ type: 'layout', ...layout });
  }, [layout, isValidLayout]);

  // Worker path: frame updates
  useEffect(() => {
    if (!SUPPORTS_OFFSCREEN || !workerRef.current || !isValidLayout) return;
    workerRef.current.postMessage({ type: 'frame', colors, isDefault });
  }, [colors, isDefault, isValidLayout]);

  // Worker cleanup
  useEffect(() => {
    if (!SUPPORTS_OFFSCREEN) return;
    return () => {
      workerRef.current?.postMessage({ type: 'dispose' });
      workerRef.current?.terminate();
      workerRef.current = null;
      workerReady.current = false;
    };
  }, []);

  // Fallback: main-thread rendering
  useEffect(() => {
    if (SUPPORTS_OFFSCREEN) return;
    const canvas = canvasRef.current;
    if (!canvas || !isValidLayout) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

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
        if (!isMatrix && i >= totalLeds) continue;

        const x = offsetX + col * (size + gap);
        const y = offsetY + row * (size + gap);

        if (isMatrix && matrixMap?.[i] === null) {
          ctx.fillStyle = EMPTY_CELL_FILL;
          drawRoundedRect(ctx, x, y, size, size, radius);
          continue;
        }

        const c = (colors?.[i] as LedColor | undefined) ?? { r: 0, g: 0, b: 0 };
        ctx.fillStyle = isDefault ? DEFAULT_FILL : `rgb(${c.r},${c.g},${c.b})`;
        drawRoundedRect(ctx, x, y, size, size, radius);
      }
    }
  }, [layout, isValidLayout, colors, isDefault, isMatrix, totalLeds, matrixMap]);

  return (
    <div ref={containerRef} style={{ width: '100%', height: '100%', overflow: 'hidden', position: 'relative' }}>
      <canvas
        ref={canvasRef}
        style={{ position: 'absolute', top: '50%', right: 0, transform: 'translateY(-50%)', display: 'block' }}
      />
    </div>
  );
}
