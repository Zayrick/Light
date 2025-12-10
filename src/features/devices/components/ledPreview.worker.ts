/// <reference lib="webworker" />

import type { LayoutData, LedColor } from './ledPreviewTypes';

type InitMessage = { type: 'init'; canvas: OffscreenCanvas; dpr: number };
type LayoutMessage = { type: 'layout' } & LayoutData;
type FrameMessage = { type: 'frame'; colors: LedColor[] | null; isDefault: boolean };
type DisposeMessage = { type: 'dispose' };
type IncomingMessage = InitMessage | LayoutMessage | FrameMessage | DisposeMessage;

let ctx: OffscreenCanvasRenderingContext2D | null = null;
let canvas: OffscreenCanvas | null = null;
let dpr = 1;
let layout: LayoutData | null = null;

const DEFAULT_FILL = 'rgba(128, 128, 128, 0.2)';
const EMPTY_CELL_FILL = 'rgba(255, 255, 255, 0.06)';

function drawRoundedRect(
  c: OffscreenCanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number
) {
  const radius = Math.min(r, w / 2, h / 2);
  c.beginPath();
  c.moveTo(x + radius, y);
  c.lineTo(x + w - radius, y);
  c.quadraticCurveTo(x + w, y, x + w, y + radius);
  c.lineTo(x + w, y + h - radius);
  c.quadraticCurveTo(x + w, y + h, x + w - radius, y + h);
  c.lineTo(x + radius, y + h);
  c.quadraticCurveTo(x, y + h, x, y + h - radius);
  c.lineTo(x, y + radius);
  c.quadraticCurveTo(x, y, x + radius, y);
  c.closePath();
  c.fill();
}

function drawFrame(colors: LedColor[] | null, isDefault: boolean) {
  if (!ctx || !canvas || !layout) return;
  const { width, height, cols, rows, gap, size, offsetX, offsetY, isMatrix, matrixMap, totalLeds } = layout;
  if (cols === 0 || rows === 0 || size <= 0) return;

  canvas.width = width * dpr;
  canvas.height = height * dpr;
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

      const c = colors?.[i] ?? { r: 0, g: 0, b: 0 };
      ctx.fillStyle = isDefault ? DEFAULT_FILL : `rgb(${c.r},${c.g},${c.b})`;
      drawRoundedRect(ctx, x, y, size, size, radius);
    }
  }
}

self.onmessage = (e: MessageEvent<IncomingMessage>) => {
  const msg = e.data;
  switch (msg.type) {
    case 'init':
      canvas = msg.canvas;
      ctx = canvas.getContext('2d');
      dpr = msg.dpr || 1;
      break;
    case 'layout':
      layout = msg;
      break;
    case 'frame':
      drawFrame(msg.colors, msg.isDefault);
      break;
    case 'dispose':
      ctx = null;
      canvas = null;
      layout = null;
      self.close();
      break;
  }
};

export {};
