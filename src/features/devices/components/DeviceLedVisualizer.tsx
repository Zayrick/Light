import { useMemo } from 'react';
import useMeasure from 'react-use-measure';
import { useLedColors } from '../../../hooks/useLedStream';

interface DeviceLedVisualizerProps {
  port: string;
  length: number;
}

export function DeviceLedVisualizer({ port, length }: DeviceLedVisualizerProps) {
  const [ref, bounds] = useMeasure();
  const { colors, isDefault } = useLedColors(port, length);

  const displayColors = useMemo(() => {
    return colors;
  }, [colors]);

  // Layout Algorithm
  const { gap, size, cols } = useMemo(() => {
    if (bounds.width === 0 || bounds.height === 0 || length === 0) {
      return { gap: 0, size: 0, cols: 0, rows: 0 };
    }

    const gap = 4; // Gap between beads
    let bestSize = 0;
    let bestCols = 1;
    let bestRows = 1;

    // Try different column counts
    // Heuristic: start from sqrt(N) to balance aspect ratio, but we must fit in WxH
    // Since W is usually > H for this header area, we might prefer more columns.
    
    // Iterate all possible column counts
    for (let c = 1; c <= length; c++) {
      const r = Math.ceil(length / c);
      
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

    return { gap, size: Math.max(0, bestSize), cols: bestCols, rows: bestRows };
  }, [bounds.width, bounds.height, length]);

  return (
    <div ref={ref} style={{ width: '100%', height: '100%', overflow: 'hidden', position: 'relative' }}>
        <div style={{
            display: 'grid',
            gridTemplateColumns: `repeat(${cols}, ${size}px)`,
            gap: `${gap}px`,
            // Center the grid in the container
            position: 'absolute',
            top: '50%',
            right: 0, // Align to right as requested
            transform: 'translateY(-50%)',
            // If we want it strictly strictly right aligned, simple 'right: 0' is fine.
            // If we want it centered if there is extra space, use left: 50%, x: -50%.
            // User said: "create a area on the right... always visible"
            // Let's align to the right side of the allocated space.
        }}>
            {displayColors.map((c, i) => {
                // Determine CSS color
                const colorStr = isDefault 
                    ? `rgba(128, 128, 128, 0.2)` 
                    : `rgb(${c.r}, ${c.g}, ${c.b})`;
                
                return (
                    <div
                        key={i}
                        style={{
                            width: size,
                            height: size,
                            backgroundColor: colorStr,
                            borderRadius: '4px', // Rounded rectangle
                            transition: 'background-color 0.1s ease'
                        }}
                    />
                );
            })}
        </div>
    </div>
  );
}
