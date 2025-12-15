import { useState, useRef, useEffect } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { AppLayout } from "./features/layout/AppLayout";
import { Sidebar } from "./features/layout/Sidebar";
import { HomePage } from "./features/home/HomePage";
import { DeviceDetail } from "./features/devices/components/DeviceDetail";
import { SettingsPage } from "./features/settings/SettingsPage";
import { useDevices } from "./hooks/useDevices";
import { useEffects } from "./hooks/useEffects";
import { PlatformProvider } from "./hooks/usePlatform";
import "./styles/theme.css";
import "./styles/layout.css";

const variants = {
  enter: (direction: number) => ({
    y: direction > 0 ? 20 : -20,
    opacity: 0,
  }),
  center: {
    y: 0,
    opacity: 1,
  },
  exit: (direction: number) => ({
    y: direction > 0 ? -20 : 20,
    opacity: 0,
  }),
};

const ANIMATION_TRANSITION = {
  duration: 0.3,
  ease: [0.16, 1, 0.3, 1] as const,
};

export default function App() {
  const [activeTab, setActiveTab] = useState("home");
  const [selectedLayoutId, setSelectedLayoutId] = useState<string | null>(null);
  const {
    devices,
    selectedDevice,
    setSelectedDevice,
    isScanning,
    scanDevices,
    updateDeviceEffect,
    updateDeviceParams,
    updateDeviceBrightness,
  } = useDevices();
  
  const { effects, applyEffect } = useEffects();

  // Calculate direction based on sidebar order
  const getPageIndex = (tab: string, deviceId: string | undefined, layoutId: string | null) => {
    if (tab === "home") return 0;
    if (tab === "settings") return 9999; // Always at bottom
    if (tab === "device-detail" && deviceId) {
      const idx = devices.findIndex(d => d.id === deviceId);
      return idx >= 0 ? idx + 1 : 0;
    }
    if (tab === "layout-preview" && layoutId) {
      // Layout items come after devices
      return devices.length + 1;
    }
    return 0;
  };

  const currentIndex = getPageIndex(activeTab, selectedDevice?.id, selectedLayoutId);
  const prevIndexRef = useRef(currentIndex);
  const direction = currentIndex > prevIndexRef.current ? 1 : -1;

  useEffect(() => {
    prevIndexRef.current = currentIndex;
  }, [currentIndex]);

  const handleSetEffect = async (port: string, effectId: string) => {
    const ok = await applyEffect(port, effectId);
    if (ok) {
      updateDeviceEffect(port, effectId);
    }
  };

  const handleNavigate = (deviceId: string) => {
    const device = devices.find((d) => d.id === deviceId);
    if (device) {
      setSelectedDevice(device);
      setActiveTab("device-detail");
    }
  };

  return (
    <PlatformProvider>
      <AppLayout
        disableScroll={activeTab === "device-detail" || activeTab === "home" || activeTab === "layout-preview"}
        hideScrollbar={activeTab === "settings"}
        pageKey={`${activeTab}-${selectedDevice?.id || selectedLayoutId || ""}`}
        sidebar={
          <Sidebar
            activeTab={activeTab}
            setActiveTab={setActiveTab}
            devices={devices}
            selectedDevice={selectedDevice}
            setSelectedDevice={setSelectedDevice}
            selectedLayoutId={selectedLayoutId}
            setSelectedLayoutId={setSelectedLayoutId}
          />
        }
      >
        <AnimatePresence mode="wait" custom={direction}>
          {activeTab === "home" && (
            <motion.div
              key="home"
              custom={direction}
              variants={variants}
              initial="enter"
              animate="center"
              exit="exit"
              transition={ANIMATION_TRANSITION}
              style={{ width: "100%", flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
            >
              <HomePage
                devices={devices}
                effects={effects}
                isScanning={isScanning}
                onScan={scanDevices}
                onNavigate={handleNavigate}
              />
            </motion.div>
          )}

          {activeTab === "device-detail" && selectedDevice && (
            <motion.div
              key={`device-detail-${selectedDevice.id}`}
              custom={direction}
              variants={variants}
              initial="enter"
              animate="center"
              exit="exit"
              transition={ANIMATION_TRANSITION}
              style={{ width: "100%", flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
            >
              <DeviceDetail
                device={selectedDevice}
                effects={effects}
                onSetEffect={handleSetEffect}
                onUpdateParams={updateDeviceParams}
                onUpdateBrightness={updateDeviceBrightness}
              />
            </motion.div>
          )}

          {activeTab === "layout-preview" && selectedLayoutId && (
            <motion.div
              key={`layout-preview-${selectedLayoutId}`}
              custom={direction}
              variants={variants}
              initial="enter"
              animate="center"
              exit="exit"
              transition={ANIMATION_TRANSITION}
              style={{ width: "100%", flex: 1, minHeight: 0, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center" }}
            >
              <div style={{ textAlign: "center", color: "var(--text-secondary)" }}>
                <h2 style={{ marginBottom: 8, color: "var(--text-primary)" }}>Layout Preview</h2>
                <p>Selected: {selectedLayoutId}</p>
              </div>
            </motion.div>
          )}

          {activeTab === "settings" && (
            <motion.div
              key="settings"
              custom={direction}
              variants={variants}
              initial="enter"
              animate="center"
              exit="exit"
              transition={ANIMATION_TRANSITION}
              style={{ width: "100%", flex: 1, display: "flex", flexDirection: "column" }}
            >
              <SettingsPage />
            </motion.div>
          )}
        </AnimatePresence>
      </AppLayout>
    </PlatformProvider>
  );
}
