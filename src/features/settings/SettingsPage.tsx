import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { getVersion as getAppVersion, getTauriVersion } from "@tauri-apps/api/app";
import { HStack, Slider } from "@chakra-ui/react";
import { Card } from "../../components/ui/Card";
import { Select } from "../../components/ui/Select";
import { Switch } from "../../components/ui/Switch";
import { api, CaptureMethod, SystemInfo, WindowEffectId } from "../../services/api";
import { configManager } from "../../services/config";
import { logger } from "../../services/logger";
import { usePlatform } from "../../hooks/usePlatform";
import { useMinimizeToTray } from "../../hooks/useMinimizeToTray";
import { useLatestThrottledInvoker } from "../../hooks/useLatestThrottledInvoker";
import {
  CAPTURE_QUALITY_PRESETS,
  DEFAULT_CAPTURE_MAX_PIXELS,
  getCaptureQualityIndex,
  getCaptureQualityPreset,
} from "../../utils/captureQuality";
import "./Settings.css";

// Windows-specific capture methods
const windowsCaptureMethodOptions = [
  { value: "dxgi" as const, label: "DXGI (Desktop Duplication)" },
  { value: "graphics" as const, label: "Graphics Capture (Windows 10+)" },
  { value: "gdi" as const, label: "GDI (Legacy)" },
];

const formatPixelBudget = (pixels: number) =>
  pixels === 0 ? "No limit" : `${pixels.toLocaleString()} px`;

// Live sync is handled via a ready/busy gate (latest-wins), so no timer-based throttling is needed.

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
  const { minimizeToTray, setMinimizeToTray } = useMinimizeToTray();
  const [captureQualityIndex, setCaptureQualityIndex] = useState<number>(
    getCaptureQualityIndex(DEFAULT_CAPTURE_MAX_PIXELS),
  );
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

  const lastSyncedQualityRef = useRef<number | null>(null);
  const lastSyncedFpsRef = useRef<number | null>(null);

  const qualityLive = useLatestThrottledInvoker<number>(
    (maxPixels) => configManager.setCaptureMaxPixels(maxPixels),
    0,
    {
      areEqual: (a, b) => a === b,
      onError: (err) => logger.error("settings.captureMaxPixels.live_failed", {}, err),
    },
  );

  const fpsLive = useLatestThrottledInvoker<number>(
    (fps) => configManager.setCaptureFps(fps),
    0,
    {
      areEqual: (a, b) => a === b,
      onError: (err) => logger.error("settings.captureFps.live_failed", {}, err),
    },
  );

  const syncLiveQuality = useCallback(
    (index: number, options?: { force?: boolean }) => {
      const preset = getCaptureQualityPreset(index);
      const maxPixels = preset.maxPixels;
      if (lastSyncedQualityRef.current === maxPixels && !options?.force) return;
      lastSyncedQualityRef.current = maxPixels;
      qualityLive.schedule(maxPixels, { force: options?.force });
    },
    [qualityLive],
  );

  const syncLiveFps = useCallback(
    (value: number, options?: { force?: boolean }) => {
      const target = Math.round(value);
      if (lastSyncedFpsRef.current === target && !options?.force) return;
      lastSyncedFpsRef.current = target;
      fpsLive.schedule(target, { force: options?.force });
    },
    [fpsLive],
  );

  useEffect(() => {
    Promise.all([configManager.getAppConfig(), api.getWindowEffects()]).then(([cfg, windowEffects]) => {
      const maxPixels = cfg.screenCapture.maxPixels;
      const fps = cfg.screenCapture.fps;
      const method = cfg.screenCapture.method;

      setCaptureFps(fps);
      lastSyncedFpsRef.current = fps;
      setCaptureMethod(method);
      setAvailableWindowEffects(windowEffects);

      if (windowEffects.length > 0) {
        const effective = windowEffects.includes(cfg.windowEffect)
          ? cfg.windowEffect
          : windowEffects[0];
        setWindowEffect(effective);
        if (effective !== cfg.windowEffect) {
          // 配置中的值不在当前支持列表中时，对齐到第一个可用选项。
          configManager.setWindowEffect(effective);
        }
      } else {
        setWindowEffect("");
      }

      const qualityIndex = getCaptureQualityIndex(maxPixels);
      const preset = getCaptureQualityPreset(qualityIndex);
      setCaptureQualityIndex(qualityIndex);
      lastSyncedQualityRef.current = preset.maxPixels;

      setLoading(false);
    });
    return () => {
      qualityLive.cancel();
      fpsLive.cancel();
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
    qualityLive.cancel();
    fpsLive.cancel();
    // Optimistic UI update; we'll reconcile with backend response below.
    setCaptureMethod(value);

    configManager
      .setCaptureMethod(value)
      .then((saved) => {
        const effective = saved.screenCapture.method;
        setCaptureMethod(effective);
      })
      .catch((err) => {
        logger.error("settings.captureMethod.update_failed", { requested: value }, err);
      });
  };

  const handleQualityChange = (value: number) => {
    setCaptureQualityIndex(value);
    syncLiveQuality(value);
  };

  const handleQualityCommit = (value: number) => {
    setCaptureQualityIndex(value);
    syncLiveQuality(value, { force: true });
  };

  const handleFpsChange = (value: number) => {
    setCaptureFps(value);
    syncLiveFps(value);
  };

  const handleFpsCommit = (value: number) => {
    setCaptureFps(value);
    syncLiveFps(value, { force: true });
  };

  const handleWindowEffectChange = (value: WindowEffectId) => {
    setWindowEffect(value);
    configManager.setWindowEffect(value);
  };

  const activeQuality = useMemo(
    () => getCaptureQualityPreset(captureQualityIndex),
    [captureQualityIndex],
  );

  // Calculate color from green (low quality = high performance) to red (high quality = low performance)
  const qualityColor = useMemo(() => {
    const maxIndex = CAPTURE_QUALITY_PRESETS.length - 1;
    // Normalize index to 0-1 range (0 = green, 1 = red)
    const ratio = captureQualityIndex / maxIndex;
    // Interpolate from green (hsl 120) to red (hsl 0)
    const hue = 120 * (1 - ratio);
    return `hsl(${hue}, 70%, 50%)`;
  }, [captureQualityIndex]);

  const displayQualityText = useCallback(() => {
    const pixelText = activeQuality.maxPixels === 0
      ? `(${formatPixelBudget(activeQuality.maxPixels)})`
      : `· ${formatPixelBudget(activeQuality.maxPixels)}`;

    return (
      <span style={{ color: qualityColor }}>
        {activeQuality.label} {pixelText}
      </span>
    );
  }, [activeQuality, qualityColor]);

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
          <h3>Window Behavior</h3>

          <div className="setting-section">
            <Switch
              checked={minimizeToTray}
              onChange={setMinimizeToTray}
              label="最小化到托盘"
              description="启用后，最小化/关闭将隐藏窗口并保留托盘入口；点击托盘图标可恢复窗口。"
            />
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

          {/* Resolution Quality Slider */}
          <div className="setting-section">
            <Slider.Root
              min={0}
              max={CAPTURE_QUALITY_PRESETS.length - 1}
              step={1}
              value={[captureQualityIndex]}
              onValueChange={(details) => handleQualityChange(details.value[0])}
              onValueChangeEnd={(details) => handleQualityCommit(details.value[0])}
              disabled={loading}
            >
              <HStack justify="space-between">
                <Slider.Label>Quality Level</Slider.Label>
                <Slider.ValueText>{displayQualityText()}</Slider.ValueText>
              </HStack>
              <Slider.Control>
                <Slider.Track>
                  <Slider.Range />
                </Slider.Track>
                <Slider.Thumbs />
              </Slider.Control>
            </Slider.Root>
            <p>
              Capture resolution is limited by a pixel budget and downscaled in power-of-two steps.
            </p>
            <p>{activeQuality.description}</p>
          </div>

          {/* Frame Rate Slider */}
          <div className="setting-section">
            <Slider.Root
              min={1}
              max={60}
              value={[captureFps]}
              onValueChange={(details) => handleFpsChange(details.value[0])}
              onValueChangeEnd={(details) => handleFpsCommit(details.value[0])}
              disabled={loading}
            >
              <HStack justify="space-between">
                <Slider.Label>Sampling Frame Rate</Slider.Label>
                <Slider.ValueText>{captureFps} FPS</Slider.ValueText>
              </HStack>
              <Slider.Control>
                <Slider.Track>
                  <Slider.Range />
                </Slider.Track>
                <Slider.Thumbs />
              </Slider.Control>
            </Slider.Root>
            <p>
              Control how often the screen is sampled per second. Lower FPS reduces CPU/GPU usage but may look less smooth.
            </p>
          </div>
        </Card>
      </div>
    </div>
  );
}
