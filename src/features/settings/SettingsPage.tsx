import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getVersion as getAppVersion, getTauriVersion } from "@tauri-apps/api/app";
import { Card } from "../../components/ui/Card";
import { Select } from "../../components/ui/Select";
import { Slider } from "../../components/ui/Slider";
import { api, CaptureMethod, SystemInfo, WindowEffectId } from "../../services/api";
import { logger } from "../../services/logger";
import { usePlatform } from "../../hooks/usePlatform";
import "./Settings.css";

// Windows-specific capture methods
const windowsCaptureMethodOptions = [
  { value: "dxgi" as const, label: "DXGI (Desktop Duplication)" },
  { value: "graphics" as const, label: "Graphics Capture (Windows 10+)" },
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

const LIVE_SYNC_INTERVAL = 90; // ms throttle for live capture scale sync

const windowEffectMeta: Record<WindowEffectId, { label: string; description: string }> = {
  // macOS materials
  appearanceBased: {
    label: "Appearance Based",
    description: "macOS 10.14+: Uses the system appearance to choose an appropriate material.",
  },
  light: {
    label: "Light",
    description: "macOS 10.14+: A light, translucent background material.",
  },
  dark: {
    label: "Dark",
    description: "macOS 10.14+: A dark, translucent background material.",
  },
  mediumLight: {
    label: "Medium Light",
    description: "macOS 10.14+: A medium-light material between Light and Dark.",
  },
  ultraDark: {
    label: "Ultra Dark",
    description: "macOS 10.14+: A very dark material suitable for HUDs.",
  },
  titlebar: {
    label: "Titlebar",
    description: "macOS 10.10+: Matches a standard window title bar.",
  },
  selection: {
    label: "Selection",
    description: "macOS 10.10+: Uses the selection highlight appearance.",
  },
  menu: {
    label: "Menu",
    description: "macOS 10.11+: Matches system menus and context menus.",
  },
  popover: {
    label: "Popover",
    description: "macOS 10.11+: Appropriate for popover-style surfaces.",
  },
  sidebar: {
    label: "Sidebar",
    description: "macOS 10.11+: Ideal for app sidebars and navigation.",
  },
  headerView: {
    label: "Header View",
    description: "macOS 10.14+: For header areas above content.",
  },
  sheet: {
    label: "Sheet",
    description: "macOS 10.14+: For modal sheets presented over a window.",
  },
  windowBackground: {
    label: "Window Background",
    description: "macOS 10.14+: Semantic material for general window backgrounds.",
  },
  hudWindow: {
    label: "HUD Window",
    description: "macOS 10.14+: Heads-up display styled material.",
  },
  fullScreenUI: {
    label: "Full Screen UI",
    description: "macOS 10.14+: For immersive full-screen interfaces.",
  },
  tooltip: {
    label: "Tooltip",
    description: "macOS 10.14+: Matches system tooltip appearance.",
  },
  contentBackground: {
    label: "Content Background",
    description: "macOS 10.14+: Neutral backdrop suitable for primary content.",
  },
  underWindowBackground: {
    label: "Under Window Background",
    description: "macOS 10.14+: For views layered under the main window background.",
  },
  underPageBackground: {
    label: "Under Page Background",
    description: "macOS 10.14+: For views layered under scrolling page content.",
  },
  // Windows effects
  mica: {
    label: "Mica",
    description: "Windows 11 only: Mica effect following system dark/light preference.",
  },
  tabbed: {
    label: "Tabbed Mica",
    description: "Windows 11 only: Tabbed Mica effect following system dark/light preference.",
  },
  blur: {
    label: "Blur",
    description:
      "Windows 10/11: Traditional blur effect. Hidden on known problematic Windows 11 builds.",
  },
  acrylic: {
    label: "Acrylic",
    description:
      "Windows 10/11: Acrylic effect. Hidden on Windows 10 v1903+ and Windows 11 build 22000.",
  },
};

export function SettingsPage() {
  const { isWindows } = usePlatform();
  const [captureScale, setCaptureScale] = useState<number>(5);
  const [captureFps, setCaptureFps] = useState<number>(30);
  const [captureMethod, setCaptureMethod] = useState<CaptureMethod>(isWindows ? "dxgi" : "xcap");
  const [loading, setLoading] = useState(true);
  const [windowEffect, setWindowEffect] = useState<WindowEffectId | "">("");
  const [availableWindowEffects, setAvailableWindowEffects] = useState<WindowEffectId[]>([]);
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [tauriVersion, setTauriVersion] = useState<string>("");
  const [appVersion, setAppVersion] = useState<string>("");

  // Only show capture method selector on Windows where multiple options exist
  const captureMethodOptions = useMemo(
    () => (isWindows ? windowsCaptureMethodOptions : []),
    [isWindows]
  );

  // Whether to show capture method selector (only when multiple options available)
  const showCaptureMethodSelector = captureMethodOptions.length > 1;

  const mipScalePoints = useMemo(buildMipScalePoints, []);
  // Both DXGI and Graphics Capture benefit from GPU-optimized mip scaling
  const isGpuAccelerated = captureMethod === "dxgi" || captureMethod === "graphics";

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
  const liveSyncTimerRef = useRef<number | null>(null);
  const pendingLiveScaleRef = useRef<number | null>(null);
  const lastSyncedScaleRef = useRef<number | null>(null);

  const cancelAnimation = () => {
    if (animationFrameRef.current !== null) {
      cancelAnimationFrame(animationFrameRef.current);
      animationFrameRef.current = null;
    }
  };

  const cancelLiveSync = () => {
    if (liveSyncTimerRef.current !== null) {
      window.clearTimeout(liveSyncTimerRef.current);
      liveSyncTimerRef.current = null;
    }
    pendingLiveScaleRef.current = null;
  };

  const easeOutExpo = (t: number) => (t === 1 ? 1 : 1 - Math.pow(2, -10 * t));

  const syncLiveScale = useCallback(
    (value: number, options?: { force?: boolean }) => {
      const target = isGpuAccelerated ? snapToMipScale(value) : value;
      pendingLiveScaleRef.current = target;

      if (options?.force) {
        cancelLiveSync();
        if (lastSyncedScaleRef.current !== target) {
          lastSyncedScaleRef.current = target;
          api.setCaptureScale(target);
        }
        return;
      }

      if (liveSyncTimerRef.current !== null) {
        return;
      }

      liveSyncTimerRef.current = window.setTimeout(() => {
        liveSyncTimerRef.current = null;
        const next = pendingLiveScaleRef.current;
        pendingLiveScaleRef.current = null;
        if (next === null || next === lastSyncedScaleRef.current) {
          return;
        }
        lastSyncedScaleRef.current = next;
        api.setCaptureScale(next);
      }, LIVE_SYNC_INTERVAL);
    },
    [isGpuAccelerated, snapToMipScale],
  );

  // 吸附到最近有效点并同步后端（无动画，用于初始化/切换模式）
  const alignToMipScale = useCallback(
    (value: number) => {
      const snapped = snapToMipScale(value);
      setCaptureScale(snapped);
      lastSyncedScaleRef.current = snapped;
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
        lastSyncedScaleRef.current = snapped;
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
          lastSyncedScaleRef.current = animationToRef.current;
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
      api.getWindowEffects(),
      api.getWindowEffect(),
    ]).then(([scale, fps, method, windowEffects, currentEffect]) => {
      setCaptureFps(fps);
      setCaptureMethod(method);
      setAvailableWindowEffects(windowEffects);

      if (windowEffects.length > 0) {
        const effective = windowEffects.includes(currentEffect) ? currentEffect : windowEffects[0];
        setWindowEffect(effective);
        if (!windowEffects.includes(currentEffect)) {
          // 后端返回的默认值不在当前支持列表中时，对齐到第一个可用选项。
          api.setWindowEffect(effective);
        }
      } else {
        setWindowEffect("");
      }

      // DXGI 模式下初始化时对齐 mip（无动画，静默纠正配置异常值）
      if (method === "dxgi") {
        alignToMipScale(scale);
      } else {
        setCaptureScale(scale);
        lastSyncedScaleRef.current = scale;
      }

      setLoading(false);
    });
    return () => {
      cancelAnimation();
      cancelLiveSync();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Load system & app info
  useEffect(() => {
    let cancelled = false;

    Promise.all([api.getSystemInfo(), getAppVersion(), getTauriVersion()])
      .then(([sysInfo, appVer, tauriVer]) => {
        if (cancelled) return;
        setSystemInfo(sysInfo);
        setAppVersion(appVer);
        setTauriVersion(tauriVer);
      })
      .catch((err) => {
        logger.error("settings.systemInfo.load_failed", {}, err);
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const handleMethodChange = (value: CaptureMethod) => {
    cancelAnimation();
    cancelLiveSync();
    setCaptureMethod(value);
    api.setCaptureMethod(value);

    // 从 GDI 切换到 DXGI 时，用动画吸附到最近有效点
    if (value === "dxgi") {
      animateToMipScale(captureScale);
    }
  };

  const handleScaleChange = (value: number) => {
    setCaptureScale(value);
    syncLiveScale(value);
  };

  const handleScaleCommit = (value: number) => {
    syncLiveScale(value, { force: true });

    // GDI 无 mipmap 约束，直接提交
    if (!isGpuAccelerated) {
      cancelAnimation();
      setCaptureScale(value);
      return;
    }

    // DXGI 使用动画吸附
    animateToMipScale(value);
  };

  const handleFpsChange = (value: number) => {
    setCaptureFps(value);
    api.setCaptureFps(value);
  };

  const handleWindowEffectChange = (value: WindowEffectId) => {
    setWindowEffect(value);
    api.setWindowEffect(value);
  };

  const displayScaleText = useCallback(() => {
    return `${formatPercent(captureScale)}%${captureScale === 100 ? " (Original)" : ""}`;
  }, [captureScale]);

  return (
    <div className="settings-page">
      <header className="page-header">
        <div>
          <h1 className="page-title">Settings</h1>
          <p className="page-subtitle">Configure application settings</p>
        </div>
      </header>
      <div className="settings-container">
        <Card className="settings-card">
          <h3>System Information</h3>
          <div className="setting-section">
            <p>
              <strong>系统版本：</strong>{" "}
              {systemInfo ? systemInfo.osPlatform : "Loading..."}
            </p>
            <p>
              <strong>系统版本号：</strong>{" "}
              {systemInfo ? systemInfo.osVersion : "Loading..."}
            </p>
            <p>
              <strong>操作系统版本：</strong>{" "}
              {systemInfo ? systemInfo.osBuild : "Loading..."}
            </p>
            <p>
              <strong>Tauri 版本：</strong> {tauriVersion || "Loading..."}
            </p>
            <p>
              <strong>软件版本：</strong> {appVersion || "Loading..."}
            </p>
          </div>
        </Card>

        <Card className="settings-card">
          <h3>Software Settings</h3>

          <div className="setting-section">
            <Select
              value={windowEffect || (availableWindowEffects[0] ?? "")}
              options={availableWindowEffects.map((id) => ({
                value: id,
                label: windowEffectMeta[id]?.label ?? id,
              }))}
              onChange={handleWindowEffectChange}
              disabled={loading || availableWindowEffects.length === 0}
              label="Background Effect"
              valueText={
                availableWindowEffects.length > 0
                  ? `${availableWindowEffects.length} options`
                  : "Not available on this platform"
              }
              placeholder={
                availableWindowEffects.length === 0
                  ? "Not available on this platform"
                  : "Select background effect"
              }
            />
            {windowEffect && windowEffectMeta[windowEffect] && (
              <p>{windowEffectMeta[windowEffect].description}</p>
            )}
          </div>
        </Card>

        <Card className="settings-card">
          <h3>Screen Capture Quality</h3>

          {/* Capture Method Select - Only shown on Windows where multiple options exist */}
          {showCaptureMethodSelector && (
            <div className="setting-section">
              <Select
                value={captureMethod}
                options={captureMethodOptions}
                onChange={handleMethodChange}
                disabled={loading}
                label="Capture Method"
                valueText={`${captureMethodOptions.length} options`}
              />
              <p>
                <strong>DXGI</strong>: High performance with GPU acceleration and HDR support.
                <br />
                <strong>Graphics Capture</strong>: Modern API for Windows 10+, event-driven with low latency.
                <br />
                <strong>GDI</strong>: Legacy mode with best compatibility for older systems.
              </p>
            </div>
          )}

          {/* Resolution Scale Slider */}
          <div className="setting-section">
            <Slider
              min={1}
              max={100}
              step={0.1}
              value={captureScale}
              onChange={handleScaleChange}
              onCommit={handleScaleCommit}
              markers={isGpuAccelerated ? mipScalePoints : undefined}
              disabled={loading}
              label="Resolution Scale"
              valueText={displayScaleText()}
            />
            <p>
              Lowering resolution improves performance. 100% matches native quality.
            </p>
            {isGpuAccelerated && (
              <p>
                Values snap to GPU-optimized levels for best performance.
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
    </div>
  );
}
