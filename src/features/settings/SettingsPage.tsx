import { useEffect, useState } from "react";
import { Card } from "../../components/ui/Card";
import { Select } from "../../components/ui/Select";
import { Slider } from "../../components/ui/Slider";
import { api, CaptureMethod } from "../../services/api";
import "./settings.css";

const captureMethodOptions = [
  { value: "dxgi" as const, label: "DXGI (Desktop Duplication)" },
  { value: "gdi" as const, label: "GDI (Legacy)" },
];

export function SettingsPage() {
  const [captureScale, setCaptureScale] = useState<number>(5);
  const [captureFps, setCaptureFps] = useState<number>(30);
  const [captureMethod, setCaptureMethod] = useState<CaptureMethod>("dxgi");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      api.getCaptureScale(),
      api.getCaptureFps(),
      api.getCaptureMethod(),
    ]).then(([scale, fps, method]) => {
      setCaptureScale(scale);
      setCaptureFps(fps);
      setCaptureMethod(method);
      setLoading(false);
    });
  }, []);

  const handleMethodChange = (value: CaptureMethod) => {
    setCaptureMethod(value);
    api.setCaptureMethod(value);
  };

  const handleScaleChange = (value: number) => {
    setCaptureScale(value);
    api.setCaptureScale(value);
  };

  const handleFpsChange = (value: number) => {
    setCaptureFps(value);
    api.setCaptureFps(value);
  };

  return (
    <>
      <header className="page-header">
        <div>
          <h1 className="page-title">Settings</h1>
          <p className="page-subtitle">Configure application settings</p>
        </div>
      </header>
      <div className="settings-container">
        <Card className="settings-card">
          <h3>Screen Capture Quality</h3>

          {/* Capture Method Select */}
          <div className="setting-section">
            <Select
              value={captureMethod}
              options={captureMethodOptions}
              onChange={handleMethodChange}
              disabled={loading}
              label="Capture Method"
              valueText="2 options"
            />
            <p>
              DXGI offers better performance with GPU acceleration and HDR support.
              GDI provides better compatibility with older systems.
            </p>
          </div>

          {/* Resolution Scale Slider */}
          <div className="setting-section">
            <Slider
              min={1}
              max={100}
              value={captureScale}
              onChange={handleScaleChange}
              disabled={loading}
              label="Resolution Scale"
              valueText={`${captureScale}%${captureScale === 100 ? " (Original)" : ""}`}
            />
            <p>
              Adjust the capture resolution. 100% maintains original quality (may affect performance).
              Lower values improve performance.
            </p>
          </div>

          {/* Frame Rate Slider */}
          <div className="setting-section">
            <Slider
              min={1}
              max={60}
              value={captureFps}
              onChange={handleFpsChange}
              disabled={loading}
              label="Sampling Frame Rate"
              valueText={`${captureFps} FPS`}
            />
            <p>
              Control how often the screen is sampled per second. Lower FPS reduces CPU/GPU usage but may look less smooth.
            </p>
          </div>
        </Card>
      </div>
    </>
  );
}
