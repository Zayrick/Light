import { Zap } from "lucide-react";
import { Device, EffectInfo } from "../../../types";
import { Card } from "../../../components/ui/Card";
import { Button } from "../../../components/ui/Button";

interface DeviceCardProps {
  device: Device;
  effects: EffectInfo[];
  onSetEffect: (port: string, effectId: string) => Promise<void>;
}

export function DeviceCard({ device, effects, onSetEffect }: DeviceCardProps) {
  return (
    <Card hoverable>
      <div className="device-header">
        <div className="device-info">
          <h3>{device.model}</h3>
          <p>{device.id}</p>
          <p style={{ fontSize: 10, opacity: 0.7 }}>{device.port}</p>
        </div>
        <div className="device-icon">
          <Zap size={20} />
        </div>
      </div>

      <div
        style={{
          margin: "12px 0",
          fontSize: 12,
          fontWeight: 600,
          color: "var(--text-secondary)",
        }}
      >
        Quick Effects
      </div>

      <div className="device-actions">
        {effects.map((effect) => (
          <Button
            key={effect.id}
            variant={device.current_effect_id === effect.id ? "primary" : "secondary"}
            style={{ fontSize: 11, padding: "4px 8px" }}
            onClick={(e) => {
              e.stopPropagation();
              onSetEffect(device.port, effect.id);
            }}
          >
            {effect.name}
          </Button>
        ))}
      </div>
    </Card>
  );
}

