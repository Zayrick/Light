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
import { pageVariants, PAGE_TRANSITION } from "./motion/transitions";
import { normalizeSelectedScope } from "./utils/scope";
import "./styles/theme.css";
import "./styles/layout.css";

export default function App() {
  const [activeTab, setActiveTab] = useState("home");
  const {
    devices,
    selectedScope,
    setSelectedScope,
    isScanning,
    scanDevices,
    refreshDevices,
  } = useDevices();
  
  const { effects } = useEffects();

  const selectedDevice = selectedScope
    ? devices.find((d) => d.port === selectedScope.port) ?? null
    : null;

  // Calculate direction based on sidebar order
  const getPageIndex = (tab: string, devicePort?: string) => {
    if (tab === "home") return 0;
    if (tab === "settings") return 9999; // Always at bottom
    if (tab === "device-detail" && devicePort) {
      const idx = devices.findIndex((d) => d.port === devicePort);
      return idx >= 0 ? idx + 1 : 0;
    }
    return 0;
  };

  const currentIndex = getPageIndex(activeTab, selectedScope?.port);
  const prevIndexRef = useRef(currentIndex);
  const direction = currentIndex > prevIndexRef.current ? 1 : -1;

  useEffect(() => {
    prevIndexRef.current = currentIndex;
  }, [currentIndex]);

  const handleNavigate = (devicePort: string) => {
    const device = devices.find((d) => d.port === devicePort);
    if (device) {
      setSelectedScope(normalizeSelectedScope({ port: device.port }, devices));
      setActiveTab("device-detail");
    }
  };

  return (
    <PlatformProvider>
      <AppLayout
        disableScroll={activeTab === "device-detail" || activeTab === "home"}
        hideScrollbar={activeTab === "settings"}
        pageKey={`${activeTab}-${selectedDevice?.id || ""}`}
        sidebar={
          <Sidebar
            activeTab={activeTab}
            setActiveTab={setActiveTab}
            devices={devices}
            selectedScope={selectedScope}
            setSelectedScope={setSelectedScope}
          />
        }
      >
        <AnimatePresence mode="wait" custom={direction}>
          {activeTab === "home" && (
            <motion.div
              key="home"
              custom={direction}
              variants={pageVariants}
              initial="enter"
              animate="center"
              exit="exit"
              transition={PAGE_TRANSITION}
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
              variants={pageVariants}
              initial="enter"
              animate="center"
              exit="exit"
              transition={PAGE_TRANSITION}
              style={{ width: "100%", flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
            >
              <DeviceDetail
                device={selectedDevice}
                scope={selectedScope ?? { port: selectedDevice.port }}
                effects={effects}
                onRefresh={refreshDevices}
              />
            </motion.div>
          )}

          {activeTab === "settings" && (
            <motion.div
              key="settings"
              custom={direction}
              variants={pageVariants}
              initial="enter"
              animate="center"
              exit="exit"
              transition={PAGE_TRANSITION}
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
