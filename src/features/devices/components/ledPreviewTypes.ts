/** Shared types for LED preview rendering (main thread & worker). */

export interface LedColor {
  r: number;
  g: number;
  b: number;
}

export interface LayoutData {
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
}

