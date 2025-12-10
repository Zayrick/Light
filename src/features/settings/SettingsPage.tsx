import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Card } from "../../components/ui/Card";
import { Select } from "../../components/ui/Select";
import { Slider } from "../../components/ui/Slider";
import { api, CaptureMethod } from "../../services/api";
import "./settings.css";

const captureMethodOptions = [
  { value: "dxgi" as const, label: "DXGI (Desktop Duplication)" },
  { value: "gdi" as const, label: "GDI (Legacy)" },
];

const buildMipScalePoints = () => {
  const points: number[] = [100];
  let current = 100;

  while (current > 1) {
    const next = Math.max(1, Math.round(current / 2));
    if (points[points.length - 1] === next) break;
    points.push(next);
    current = next;
  }

  return points;
};

const formatPercent = (value: number) =>
  Number.isInteger(value) ? value.toString() : value.toFixed(1);

export function SettingsPage() {
  const [captureScale, setCaptureScale] = useState<number>(5);
  const [captureFps, setCaptureFps] = useState<number>(30);
  const [captureMethod, setCaptureMethod] = useState<CaptureMethod>("dxgi");
  const [loading, setLoading] = useState(true);

  const mipScalePoints = useMemo(buildMipScalePoints, []);
  const isDxgi = captureMethod === "dxgi";

  const snapToMipScale = useCallback(
    (value: number) =>
      mipScalePoints.reduce(
        (closest, point) =>
          Math.abs(point - value) < Math.abs(closest - value) ? point : closest,
        mipScalePoints[0],
      ),
    [mipScalePoints],
  );

  const animationFrameRef = useRef<number | null>(null);
  const animationStartRef = useRef<number>(0);
  const animationFromRef = useRef<number>(0);
  const animationToRef = useRef<number>(0);
  const animationDuration = 220; // ms

  const cancelAnimation = () => {
    if (animationFrameRef.current !== null) {
      cancelAnimationFrame(animationFrameRef.current);
      animationFrameRef.current = null;
    }
  };

  const easeOutExpo = (t: number) => (t === 1 ? 1 : 1 - Math.pow(2, -10 * t));

  // 吸附到最近有效点并同步后端（无动画，用于初始化/切换模式）
  const alignToMipScale = useCallback(
    (value: number) => {
      const snapped = snapToMipScale(value);
      setCaptureScale(snapped);
      if (snapped !== value) {
        api.setCaptureScale(snapped);
      }
    },
    [snapToMipScale],
  );

  // 带动画吸附到最近有效点
  const animateToMipScale = useCallback(
    (from: number) => {
      const snapped = snapToMipScale(from);

      // 已经在有效点上
      if (Math.abs(snapped - from) < 0.01) {
        setCaptureScale(snapped);
        api.setCaptureScale(snapped);
        return;
      }

      cancelAnimation();
      animationStartRef.current = performance.now();
      animationFromRef.current = from;
      animationToRef.current = snapped;

      const step = (now: number) => {
        const elapsed = now - animationStartRef.current;
        const t = Math.min(1, elapsed / animationDuration);
        const eased = easeOutExpo(t);
        const next =
          animationFromRef.current +
          (animationToRef.current - animationFromRef.current) * eased;
        setCaptureScale(next);

        if (t < 1) {
          animationFrameRef.current = requestAnimationFrame(step);
        } else {
          animationFrameRef.current = null;
          setCaptureScale(animationToRef.current);
          api.setCaptureScale(animationToRef.current);
        }
      };

      animationFrameRef.current = requestAnimationFrame(step);
    },
    [snapToMipScale, animationDuration],
  );

  useEffect(() => {
    Promise.all([
      api.getCaptureScale(),
      api.getCaptureFps(),
      api.getCaptureMethod(),
    ]).then(([scale, fps, method]) => {
      setCaptureFps(fps);
      setCaptureMethod(method);

      // DXGI 模式下初始化时对齐 mip（无动画，静默纠正配置异常值）
      if (method === "dxgi") {
        alignToMipScale(scale);
      } else {
        setCaptureScale(scale);
      }

      setLoading(false);
    });
    return () => cancelAnimation();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleMethodChange = (value: CaptureMethod) => {
    cancelAnimation();
    setCaptureMethod(value);
    api.setCaptureMethod(value);

    // 从 GDI 切换到 DXGI 时，用动画吸附到最近有效点
    if (value === "dxgi") {
      animateToMipScale(captureScale);
    }
  };

  const handleScaleChange = (value: number) => {
    setCaptureScale(value);
  };

  const handleScaleCommit = (value: number) => {
    // GDI 无 mipmap 约束，直接提交
    if (!isDxgi) {
      cancelAnimation();
      setCaptureScale(value);
      api.setCaptureScale(value);
      return;
    }

    // DXGI 使用动画吸附
    animateToMipScale(value);
  };

  const handleFpsChange = (value: number) => {
    setCaptureFps(value);
    api.setCaptureFps(value);
  };

  const displayScaleText = useCallback(() => {
    return `${formatPercent(captureScale)}%${captureScale === 100 ? " (Original)" : ""}`;
  }, [captureScale]);

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
              step={0.1}
              value={captureScale}
              onChange={handleScaleChange}
              onCommit={handleScaleCommit}
              markers={isDxgi ? mipScalePoints : undefined}
              disabled={loading}
              label="Resolution Scale"
              valueText={displayScaleText()}
            />
            <p>
              Adjust the capture resolution. 100% maintains original quality (may affect performance).
              Lower values improve performance.
            </p>
            {isDxgi && (
              <p>
                Mipmap snap points: {mipScalePoints.map((p) => `${p}%`).join(" / ")}. Values will snap to
                the nearest point when released to match GPU downsampling levels.
              </p>
            )}
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
