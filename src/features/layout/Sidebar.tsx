import { Monitor, Settings, Zap } from "lucide-react";
import clsx from "clsx";
import { motion } from "framer-motion";
import { Device } from "../../types";

interface SidebarProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
  devices: Device[];
  selectedDevice: Device | null;
  setSelectedDevice: (device: Device | null) => void;
  statusMsg: string;
}

export function Sidebar({
  activeTab,
  setActiveTab,
  devices,
  selectedDevice,
  setSelectedDevice,
  statusMsg,
}: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="sidebar-content">
        <div className="nav-group nav-group-main">
          <div>
            <div
              className={clsx("nav-item", activeTab === "devices" && "active")}
              onClick={() => setActiveTab("devices")}
            >
              {activeTab === "devices" && (
                <motion.div
                  layoutId="active-nav"
                  className="active-highlight"
                  transition={{
                    duration: 0.3,
                    ease: [0.16, 1, 0.3, 1],
                  }}
                />
              )}
              <Monitor size={18} />
              <span>Devices</span>
            </div>
            <div className="nav-divider"></div>
          </div>
          <div className="device-list">
            {devices.map((device) => (
              <div
                key={device.id}
                className={clsx(
                  "device-list-item",
                  activeTab === "device-detail" &&
                    selectedDevice?.id === device.id &&
                    "active"
                )}
                onClick={() => {
                  setSelectedDevice(device);
                  setActiveTab("device-detail");
                }}
              >
                {activeTab === "device-detail" &&
                  selectedDevice?.id === device.id && (
                    <motion.div
                      layoutId="active-nav"
                      className="active-highlight"
                      transition={{
                        duration: 0.3,
                        ease: [0.16, 1, 0.3, 1],
                      }}
                    />
                  )}
                <Zap size={18} className="device-list-icon" />
                <div className="device-list-info">
                  <div className="device-list-item-name">{device.model}</div>
                  <div className="device-list-item-port">{device.port}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
        <div className="nav-group nav-group-settings">
          <div
            className={clsx("nav-item", activeTab === "settings" && "active")}
            onClick={() => setActiveTab("settings")}
          >
            {activeTab === "settings" && (
              <motion.div
                layoutId="active-nav"
                className="active-highlight"
                transition={{
                  duration: 0.3,
                  ease: [0.16, 1, 0.3, 1],
                }}
              />
            )}
            <Settings size={18} />
            <span>Settings</span>
          </div>
        </div>
      </div>

      <div className="status-bar">{statusMsg}</div>
    </aside>
  );
}

