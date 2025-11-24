import { useEffect, useState } from "react";
import { Card } from "../../components/ui/Card";
import { api } from "../../services/api";

export function SettingsPage() {
  const [captureScale, setCaptureScale] = useState<number>(5);
  const [captureFps, setCaptureFps] = useState<number>(30);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([api.getCaptureScale(), api.getCaptureFps()]).then(([scale, fps]) => {
      setCaptureScale(scale);
      setCaptureFps(fps);
      setLoading(false);
    });
  }, []);

  const handleScaleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setCaptureScale(Number(e.target.value));
  };

  const handleScaleCommit = () => {
    api.setCaptureScale(captureScale);
  };

  const handleFpsChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setCaptureFps(Number(e.target.value));
  };

  const handleFpsCommit = () => {
    api.setCaptureFps(captureFps);
  };

  return (
    <>
      <header className="page-header">
        <div>
          <h1 className="page-title">Settings</h1>
          <p className="page-subtitle">Configure application settings</p>
        </div>
      </header>
      <div style={{ padding: "20px" }}>
        <Card style={{ padding: "20px", display: "flex", flexDirection: "column", gap: "10px" }}>
          <h3>Screen Capture Quality</h3>
          <div>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "10px" }}>
              <label htmlFor="capture-scale">Resolution Scale</label>
              <span>{captureScale}% {captureScale === 100 && "(Original)"}</span>
            </div>
            <input
              id="capture-scale"
              type="range"
              min="1"
              max="100"
              value={captureScale}
              onChange={handleScaleChange}
              onMouseUp={handleScaleCommit}
              onTouchEnd={handleScaleCommit}
              style={{ width: "100%" }}
              disabled={loading}
            />
            <p style={{ fontSize: "0.9em", color: "var(--text-secondary)", marginTop: "10px" }}>
              Adjust the capture resolution. 100% maintains original quality (may affect performance).
              Lower values improve performance.
            </p>
          </div>

          <div style={{ marginTop: "20px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "10px" }}>
              <label htmlFor="capture-fps">Sampling Frame Rate</label>
              <span>{captureFps} FPS</span>
            </div>
            <input
              id="capture-fps"
              type="range"
              min="1"
              max="60"
              value={captureFps}
              onChange={handleFpsChange}
              onMouseUp={handleFpsCommit}
              onTouchEnd={handleFpsCommit}
              style={{ width: "100%" }}
              disabled={loading}
            />
            <p style={{ fontSize: "0.9em", color: "var(--text-secondary)", marginTop: "10px" }}>
              Control how often the screen is sampled per second. Lower FPS reduces CPU/GPU usage but may look less smooth.
            </p>
          </div>
        </Card>
      </div>
    </>
  );
}
