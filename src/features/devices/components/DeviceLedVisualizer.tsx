import { useEffect, useMemo, useRef } from 'react';
import useMeasure from 'react-use-measure';
import { useLedColors } from '../../../hooks/useLedStream';
import { Device } from '../../../types';

interface DeviceLedVisualizerProps {
  device: Device;
}

export function DeviceLedVisualizer({ device }: DeviceLedVisualizerProps) {
  const [ref, bounds] = useMeasure();
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  
  const virtualWidth = device.virtual_layout ? device.virtual_layout[0] : device.length;
  const virtualHeight = device.virtual_layout ? device.virtual_layout[1] : 1;
  const totalVirtualLeds = virtualWidth * virtualHeight;

  // Use virtual length for colors array size
  const { colors, isDefault } = useLedColors(device.port, totalVirtualLeds);

  const isMatrix = virtualHeight > 1;
  const matrixZone = device.zones.find(z => z.zone_type === 'Matrix');

  // Layout Algorithm
  const { gap, size, cols, rows, matrixMap } = useMemo(() => {
    if (bounds.width === 0 || bounds.height === 0 || totalVirtualLeds === 0) {
      return { gap: 0, size: 0, cols: 0, rows: 0, matrixMap: null };
    }

    // Dynamic Gap Logic
    let gap = 2;
    if (isMatrix) {
        const maxDim = Math.max(virtualWidth, virtualHeight);
        if (maxDim > 40) gap = 0;
        else if (maxDim > 20) gap = 1;
    } else {
        // Linear
        if (totalVirtualLeds > 100) gap = 1;
    }

    if (isMatrix) {
        // Fixed Grid Logic
        const cols = virtualWidth;
        const rows = virtualHeight;
        
        // Calculate max size that fits in bounds
        // wAvailable = bounds.width - (cols - 1) * gap
        // hAvailable = bounds.height - (rows - 1) * gap
        // size = min(wAvailable / cols, hAvailable / rows)
        
        const wAvailable = bounds.width - (cols - 1) * gap;
        const hAvailable = bounds.height - (rows - 1) * gap;
        
        if (wAvailable <= 0 || hAvailable <= 0) {
             return { gap, size: 0, cols, rows, matrixMap: matrixZone?.matrix?.map };
        }

        const s = Math.min(wAvailable / cols, hAvailable / rows);
        return { gap, size: Math.max(0, s), cols, rows, matrixMap: matrixZone?.matrix?.map };

    } else {
        // Smart Wrapping Logic (for Linear)
        let bestSize = 0;
        let bestCols = 1;
        let bestRows = 1;
    
        // Iterate all possible column counts
        // For linear strip, totalVirtualLeds is the count.
        for (let c = 1; c <= totalVirtualLeds; c++) {
          const r = Math.ceil(totalVirtualLeds / c);
          
          const wAvailable = bounds.width - (c - 1) * gap;
          const hAvailable = bounds.height - (r - 1) * gap;
          
          if (wAvailable <= 0 || hAvailable <= 0) continue;
    
          const s = Math.min(wAvailable / c, hAvailable / r);
          
          if (s > bestSize) {
            bestSize = s;
            bestCols = c;
            bestRows = r;
          }
        }
        return { gap, size: Math.max(0, bestSize), cols: bestCols, rows: bestRows, matrixMap: null };
    }
  }, [bounds.width, bounds.height, totalVirtualLeds, isMatrix, virtualWidth, virtualHeight, matrixZone]);

  // Imperative canvas rendering to avoid large DOM trees and heavy React diffs.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    if (bounds.width === 0 || bounds.height === 0) return;
    if (cols === 0 || rows === 0 || size <= 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const width = bounds.width;
    const height = bounds.height;

    // Configure canvas pixel size for crisp rendering
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);

    const totalGridWidth = cols * size + (cols - 1) * gap;
    const totalGridHeight = rows * size + (rows - 1) * gap;

    // Align grid to the right and center vertically (matching previous CSS grid behavior)
    const offsetX = Math.max(0, width - totalGridWidth);
    const offsetY = Math.max(0, (height - totalGridHeight) / 2);

    const drawRoundedRect = (x: number, y: number, w: number, h: number, r: number) => {
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
    };

    const defaultFill = 'rgba(128, 128, 128, 0.2)';
    const emptyCellFill = 'rgba(255, 255, 255, 0.06)';

    for (let row = 0; row < rows; row++) {
      for (let col = 0; col < cols; col++) {
        const i = row * cols + col;
        // For linear layouts, skip cells beyond the LED count.
        if (!isMatrix && i >= totalVirtualLeds) {
          continue;
        }

        const x = offsetX + col * (size + gap);
        const y = offsetY + row * (size + gap);
        const radius = isMatrix ? Math.min(2, size / 2) : Math.min(4, size / 2);

        // For matrix layouts, show cells that have no physical LED as a faint placeholder.
        if (isMatrix && matrixMap && matrixMap[i] === null) {
          ctx.fillStyle = emptyCellFill;
          drawRoundedRect(x, y, size, size, radius);
          continue;
        }

        const c = colors && colors[i] ? colors[i] : { r: 0, g: 0, b: 0 };
        const colorStr = isDefault
          ? defaultFill
          : `rgb(${c.r ?? 0}, ${c.g ?? 0}, ${c.b ?? 0})`;

        ctx.fillStyle = colorStr;
        drawRoundedRect(x, y, size, size, radius);
      }
    }
  }, [
    bounds.width,
    bounds.height,
    cols,
    rows,
    size,
    gap,
    matrixMap,
    isMatrix,
    totalVirtualLeds,
    colors,
    isDefault,
  ]);

  return (
    <div
      ref={ref}
      style={{ width: '100%', height: '100%', overflow: 'hidden', position: 'relative' }}
    >
      <canvas
        ref={canvasRef}
        style={{
          position: 'absolute',
          top: '50%',
          right: 0,
          transform: 'translateY(-50%)',
          display: 'block',
        }}
      />
    </div>
  );
}
