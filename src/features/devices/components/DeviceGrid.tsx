import { Monitor, RefreshCw } from "lucide-react";
import clsx from "clsx";
import { Device, EffectInfo } from "../../../types";
import { DeviceCard } from "./DeviceCard";
import { Button } from "../../../components/ui/Button";

interface DeviceGridProps {
  devices: Device[];
  effects: EffectInfo[];
  isScanning: boolean;
  onScan: () => void;
  onSetEffect: (port: string, effectId: string) => void;
}

export function DeviceGrid({
  devices,
  effects,
  isScanning,
  onScan,
  onSetEffect,
}: DeviceGridProps) {
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
        <div className="devices-grid">
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

