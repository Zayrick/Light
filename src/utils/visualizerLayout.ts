import { Device, SegmentType, SelectedScope } from "../types";

export interface ProcessedZone {
  id: string; // outputId or segmentId
  outputId: string;
  outputName: string;
  segmentId?: string;
  type: SegmentType;
  name: string;
  ledStartIndex: number;
  ledCount: number;
  cols: number;
  rows: number;
  matrixMap: (number | null)[] | null;
  isMatrix: boolean;
}

/**
 * Pre-process device outputs into a flat list of zones (flattening segments).
 * Returns the zones and total LED count.
 */
export function processDeviceZones(device: Device): { zones: ProcessedZone[]; totalLeds: number } {
  let currentOffset = 0;
  const zones: ProcessedZone[] = [];

  for (const o of device.outputs) {
    // If linear and has segments, treat each segment as a zone
    if (o.output_type === 'Linear' && o.segments.length > 0) {
      for (const s of o.segments) {
        const count = s.leds_count || 0;
        const cols = Math.ceil(Math.sqrt(count)) || 0;
        const rows = cols > 0 ? Math.ceil(count / cols) : 0;

        zones.push({
          id: s.id,
          outputId: o.id,
          outputName: o.name,
          segmentId: s.id,
          type: s.segment_type,
          name: s.name,
          ledStartIndex: currentOffset,
          ledCount: count,
          cols,
          rows,
          matrixMap: null,
          isMatrix: false,
        });
        currentOffset += count;
      }
    } else {
      // Single output (Matrix or Linear without segments)
      const count = o.leds_count || 0;
      const isMatrix = o.output_type === 'Matrix' && !!o.matrix && o.matrix.width > 0 && o.matrix.height > 0;
      let cols = 0, rows = 0;

      if (isMatrix && o.matrix) {
        cols = o.matrix.width;
        rows = o.matrix.height;
      } else if (count > 0) {
        cols = Math.ceil(Math.sqrt(count));
        rows = Math.ceil(count / cols);
      }

      zones.push({
        id: o.id,
        outputId: o.id,
        outputName: o.name,
        type: o.output_type,
        name: o.name,
        ledStartIndex: currentOffset,
        ledCount: count,
        cols,
        rows,
        matrixMap: isMatrix ? (o.matrix?.map ?? null) : null,
        isMatrix,
      });
      currentOffset += count;
    }
  }

  return { zones, totalLeds: Math.max(1, currentOffset) };
}

/**
 * Filter zones based on the current scope selection.
 */
export function filterVisibleZones(zones: ProcessedZone[], scope?: SelectedScope): ProcessedZone[] {
  if (!scope?.outputId) return zones;
  if (scope.segmentId) {
    return zones.filter((z) => z.outputId === scope.outputId && z.segmentId === scope.segmentId);
  }
  return zones.filter((z) => z.outputId === scope.outputId);
}

export interface BlockLayout {
  zoneIndex: number;
  outputId: string;
  segmentId?: string;
  label: string;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  cols: number;
  rows: number;
  ledStartIndex: number;
  ledCount: number;
  matrixMap: (number | null)[] | null;
  isMatrix: boolean;
  isActive: boolean;
}

export interface MultiLayoutData {
  width: number;
  height: number;
  size: number;
  gap: number;
  blockGap: number;
  blocks: BlockLayout[];
}

export function computeMultiLayout(
  width: number,
  height: number,
  zones: ProcessedZone[],
  scope?: SelectedScope
): MultiLayoutData {
  if (width === 0 || height === 0 || zones.length === 0) {
    return { width, height, size: 0, gap: 0, blockGap: 0, blocks: [] };
  }

  const gap = 1;
  const blockGap = 16; // gap between blocks in the same row
  const rowGap = 12; // gap between rows

  const highlightEnabled = zones.length > 1;
  const outputCount = new Set(zones.map((z) => z.outputId)).size;
  const multiOutput = outputCount > 1;

  // Hover/active rounded frame can extend outside the LED grid; only reserve
  // this space when we actually show hover/active UI (multi-zone only).
  const highlightPad = highlightEnabled ? 6 : 0;
  const leftPadding = highlightPad + 4;
  const rightPadding = highlightPad + 8;
  const topPadding = highlightPad + 8;
  const bottomPadding = highlightPad + 4;

  const contentW = Math.max(0, width - leftPadding - rightPadding);
  const contentH = Math.max(0, height - topPadding - bottomPadding);

  // When user drills into a specific output/segment, prefer using the current
  // viewport aspect ratio to decide how a Linear strip should wrap. In the
  // overview (no output selected), we keep the precomputed "square-ish" grid
  // to make multi-zone comparison more consistent.
  const detailMode = !!scope?.outputId || !!scope?.segmentId;
  const viewportAspect = contentH > 0 ? contentW / contentH : 1;

  const computeLinearGrid = (count: number): { cols: number; rows: number } => {
    if (count <= 0) return { cols: 0, rows: 0 };
    // Heuristic: cols ~ sqrt(N * aspect). Wider areas -> more cols.
    const cols = Math.max(
      1,
      Math.min(
        count,
        Math.ceil(Math.sqrt(count * Math.max(0.15, viewportAspect)))
      )
    );
    const rows = Math.ceil(count / cols);
    return { cols, rows };
  };

  type Row = {
    zones: { idx: number; w: number; h: number; cols: number; rows: number }[];
    width: number;
    height: number;
  };

  const buildRows = (
    size: number
  ): { rows: Row[]; totalHeight: number } | null => {
    if (size <= 0) return null;

    const rows: Row[] = [];
    let current: Row = { zones: [], width: 0, height: 0 };

    for (let i = 0; i < zones.length; i++) {
      const z = zones[i];

      // In detail mode, reflow Linear zones based on current viewport.
      // Matrix zones keep their fixed dimensions.
      let gridCols = z.cols;
      let gridRows = z.rows;
      if (!z.isMatrix && detailMode && z.ledCount > 0) {
        const g = computeLinearGrid(z.ledCount);
        gridCols = g.cols;
        gridRows = g.rows;
      }

      const w = gridCols > 0 ? size * gridCols + gap * (gridCols - 1) : 0;
      const h = gridRows > 0 ? size * gridRows + gap * (gridRows - 1) : 0;
      if (w > contentW) return null;

      const addW = current.zones.length === 0 ? w : blockGap + w;
      if (current.zones.length > 0 && current.width + addW > contentW) {
        rows.push(current);
        current = { zones: [], width: 0, height: 0 };
      }

      if (current.zones.length === 0) {
        current.width = w;
      } else {
        current.width += blockGap + w;
      }
      current.height = Math.max(current.height, h);
      current.zones.push({ idx: i, w, h, cols: gridCols, rows: gridRows });
    }

    if (current.zones.length > 0) rows.push(current);

    const totalHeight =
      rows.reduce((acc, r) => acc + r.height, 0) +
      (rows.length > 1 ? rowGap * (rows.length - 1) : 0);
    return { rows, totalHeight };
  };

  // Binary search the largest size that fits (wrapping allowed).
  // 18 iterations gives us high precision (better than 0.01px) for the size.
  const maxCandidate = Math.min(64, Math.max(1, Math.min(contentW, contentH)));
  let lo = 0;
  let hi = maxCandidate;
  let best: { size: number; rows: Row[] } | null = null;

  for (let iter = 0; iter < 18; iter++) {
    const mid = (lo + hi) / 2;
    const built = buildRows(mid);
    if (built && built.totalHeight <= contentH) {
      best = { size: mid, rows: built.rows };
      lo = mid;
    } else {
      hi = mid;
    }
  }

  const chosen = best;
  const size = chosen ? Math.floor(chosen.size * 2) / 2 : 0; // stabilize rendering (0.5px steps)

  const rowsBuilt = chosen ? buildRows(size) : null;
  if (!rowsBuilt) {
    return { width, height, size: 0, gap, blockGap, blocks: [] };
  }

  const blocks: BlockLayout[] = [];
  let y = topPadding;

  for (const row of rowsBuilt.rows) {
    // Row is right-aligned.
    let x = width - rightPadding - row.width;

    for (const item of row.zones) {
      const z = zones[item.idx];

      let isActive = false;
      if (highlightEnabled && scope) {
        if (scope.segmentId) {
          isActive =
            z.segmentId === scope.segmentId && z.outputId === scope.outputId;
        } else if (scope.outputId && multiOutput) {
          // Only highlight output-level selection when multiple outputs are visible.
          // In a single-output drill-down view, highlighting every block is noisy.
          isActive = z.outputId === scope.outputId;
        }
      }

      const label = z.segmentId ? z.name : z.outputName;
      const title = z.segmentId ? `${z.outputName} / ${z.name}` : z.outputName;

      blocks.push({
        zoneIndex: item.idx,
        outputId: z.outputId,
        segmentId: z.segmentId,
        label,
        title,
        x,
        y,
        width: item.w,
        height: item.h,
        cols: item.cols,
        rows: item.rows,
        ledStartIndex: z.ledStartIndex,
        ledCount: z.ledCount,
        matrixMap: z.matrixMap,
        isMatrix: z.isMatrix,
        isActive,
      });

      x += item.w + blockGap;
    }

    y += row.height + rowGap;
  }

  return {
    width,
    height,
    size,
    gap,
    blockGap,
    blocks,
  };
}
