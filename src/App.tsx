import { useState } from "react";
import { AppLayout } from "./features/layout/AppLayout";
import { Sidebar } from "./features/layout/Sidebar";
import { DeviceGrid } from "./features/devices/components/DeviceGrid";
import { DeviceDetail } from "./features/devices/components/DeviceDetail";
import { SettingsPage } from "./features/settings/SettingsPage";
import { useDevices } from "./hooks/useDevices";
import { useEffects } from "./hooks/useEffects";
import "./styles/theme.css";
import "./styles/layout.css";

export default function App() {
  const [activeTab, setActiveTab] = useState("devices");
  const {
    devices,
    selectedDevice,
    setSelectedDevice,
    isScanning,
    statusMsg,
    scanDevices,
  } = useDevices();
  
  const { effects, applyEffect } = useEffects();

  const handleSetEffect = async (port: string, effectId: string) => {
    await applyEffect(port, effectId);
  };

  return (
    <AppLayout
      sidebar={
        <Sidebar
          activeTab={activeTab}
          setActiveTab={setActiveTab}
          devices={devices}
          selectedDevice={selectedDevice}
          setSelectedDevice={setSelectedDevice}
          statusMsg={statusMsg}
        />
      }
    >
      {activeTab === "devices" && (
        <DeviceGrid
          devices={devices}
          effects={effects}
          isScanning={isScanning}
          onScan={scanDevices}
          onSetEffect={handleSetEffect}
        />
      )}

      {activeTab === "device-detail" && selectedDevice && (
        <DeviceDetail device={selectedDevice} />
      )}

      {activeTab === "settings" && <SettingsPage />}
    </AppLayout>
  );
}
