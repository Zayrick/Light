import { useMemo } from 'react';
import useMeasure from 'react-use-measure';
import { useLedColors } from '../../../hooks/useLedStream';
import { Device, ZoneType } from '../../../types';

interface DeviceLedVisualizerProps {
  device: Device;
}

export function DeviceLedVisualizer({ device }: DeviceLedVisualizerProps) {
  const [ref, bounds] = useMeasure();
  
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

  // Prepare display colors
  // If colors array is shorter than expected (e.g. backend sends partial update?), pad it?
  // useLedColors already handles padding with gray if null, but if array is short?
  // We assume it matches.

  return (
    <div ref={ref} style={{ width: '100%', height: '100%', overflow: 'hidden', position: 'relative' }}>
        <div style={{
            display: 'grid',
            gridTemplateColumns: `repeat(${cols}, ${size}px)`,
            gridTemplateRows: `repeat(${rows}, ${size}px)`,
            gap: `${gap}px`,
            // Align to right
            position: 'absolute',
            top: '50%',
            right: 0,
            transform: 'translateY(-50%)',
            justifyContent: 'end',
            alignContent: 'center'
        }}>
            {Array.from({ length: cols * rows }).map((_, i) => {
                // Determine if we should render this cell
                let shouldRender = true;
                
                if (isMatrix) {
                     // In matrix mode, i is the index in the flattened grid (row-major)
                     // Check matrixMap if available
                     if (matrixMap && matrixMap[i] === null) { // Wait, backend sends None as null in JSON
                         shouldRender = false;
                     }
                } else {
                    // Linear mode: we only render up to totalVirtualLeds
                    if (i >= totalVirtualLeds) shouldRender = false;
                }

                if (!shouldRender) {
                    return <div key={i} style={{ width: size, height: size }} />;
                }

                // Get color
                // For matrix: colors[i] matches the virtual layout index
                // For linear: colors[i] matches the linear index
                const c = colors && colors[i] ? colors[i] : { r: 0, g: 0, b: 0 };
                
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
                            borderRadius: isMatrix ? '2px' : '4px', // Sharper for matrix?
                            transition: 'background-color 0.05s ease', // Faster transition
                            boxShadow: isDefault ? 'none' : `0 0 ${size/2}px ${colorStr}`, // Glow effect
                        }}
                    />
                );
            })}
        </div>
    </div>
  );
}
