import { Monitor, RefreshCw } from "lucide-react";
import clsx from "clsx";
import { useRef } from "react";
import { Device, EffectInfo } from "../../types";
import { DeviceCard } from "../devices/components/DeviceCard";
import { Button } from "../../components/ui/Button";

interface HomePageProps {
  devices: Device[];
  effects: EffectInfo[];
  isScanning: boolean;
  onScan: () => void;
  onSetEffect: (port: string, effectId: string) => void;
}

export function HomePage({
  devices,
  effects,
  isScanning,
  onScan,
  onSetEffect,
}: HomePageProps) {
  const gridRef = useRef<HTMLDivElement>(null);

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!gridRef.current) return;

    const cards = gridRef.current.getElementsByClassName("device-card");
    
    for (const card of cards) {
      const rect = card.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      
      (card as HTMLElement).style.setProperty("--mouse-x", `${x}px`);
      (card as HTMLElement).style.setProperty("--mouse-y", `${y}px`);
      (card as HTMLElement).style.setProperty("--spotlight-opacity", "1");
    }
  };

  return (
    <>
      <header className="page-header">
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <div>
            <h1 className="page-title">Connected Devices</h1>
            <p className="page-subtitle">
              Manage your lighting devices and effects
            </p>
          </div>
          <Button onClick={onScan} disabled={isScanning}>
            <RefreshCw size={16} className={clsx(isScanning && "animate-spin")} />
            Scan Devices
          </Button>
        </div>
      </header>

      {devices.length === 0 && !isScanning ? (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            height: "50%",
            color: "var(--text-secondary)",
          }}
        >
          <Monitor size={48} style={{ marginBottom: 16, opacity: 0.3 }} />
          <p>No devices connected</p>
          <Button variant="secondary" style={{ marginTop: 16 }} onClick={onScan}>
            Try Again
          </Button>
        </div>
      ) : (
        <div 
          ref={gridRef}
          className="devices-grid" 
          onMouseMove={handleMouseMove}
          onMouseLeave={() => {
            // Optional: reset or fade out effects when leaving grid
            if (gridRef.current) {
              const cards = gridRef.current.getElementsByClassName("device-card");
              for (const card of cards) {
                // Handle via CSS opacity for smooth fade out
                (card as HTMLElement).style.setProperty("--spotlight-opacity", "0");
              }
            }
          }}
        >
          {devices.map((dev, idx) => (
            <DeviceCard
              key={`${dev.id}-${idx}`}
              device={dev}
              effects={effects}
              onSetEffect={onSetEffect}
            />
          ))}
        </div>
      )}
    </>
  );
}

