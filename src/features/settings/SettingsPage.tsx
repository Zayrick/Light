import { useEffect, useState } from "react";
import { Card } from "../../components/ui/Card";
import { api } from "../../services/api";

export function SettingsPage() {
  const [captureScale, setCaptureScale] = useState<number>(5);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.getCaptureScale().then((scale) => {
      setCaptureScale(scale);
      setLoading(false);
    });
  }, []);

  const handleScaleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setCaptureScale(Number(e.target.value));
  };

  const handleScaleCommit = () => {
    api.setCaptureScale(captureScale);
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
        </Card>
      </div>
    </>
  );
}
