import { Home, Settings, Zap } from "lucide-react";
import clsx from "clsx";
import { motion } from "framer-motion";
import { Device } from "../../types";

const NAV_TRANSITION = {
  duration: 0.25,
  ease: [0.16, 1, 0.3, 1] as const,
};

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
  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    e.currentTarget.style.setProperty("--mouse-x", `${x}px`);
    e.currentTarget.style.setProperty("--mouse-y", `${y}px`);
    e.currentTarget.style.setProperty("--spotlight-opacity", "1");
  };

  const handleMouseLeave = (e: React.MouseEvent<HTMLDivElement>) => {
    e.currentTarget.style.setProperty("--spotlight-opacity", "0");
  };

  return (
    <aside className="sidebar">
      <div className="sidebar-content">
        <div className="nav-group nav-group-main">
          <div>
            <motion.div
              className={clsx("nav-item", activeTab === "home" && "active")}
              onClick={() => setActiveTab("home")}
              onMouseMove={handleMouseMove}
              onMouseLeave={handleMouseLeave}
              animate={{
                fontWeight: activeTab === "home" ? 600 : 400,
              }}
              transition={NAV_TRANSITION}
            >
              {activeTab === "home" && (
                <motion.div
                  layoutId="active-nav"
                  className="active-highlight"
                  transition={{
                    duration: 0.3,
                    ease: [0.16, 1, 0.3, 1],
                  }}
                />
              )}
              <Home size={18} />
              <span>Home</span>
            </motion.div>
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
                onMouseMove={handleMouseMove}
                onMouseLeave={handleMouseLeave}
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
          <motion.div
            className={clsx("nav-item", activeTab === "settings" && "active")}
            onClick={() => setActiveTab("settings")}
            onMouseMove={handleMouseMove}
            onMouseLeave={handleMouseLeave}
            animate={{
              fontWeight: activeTab === "settings" ? 600 : 400,
            }}
            transition={NAV_TRANSITION}
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
          </motion.div>
        </div>
      </div>

      <div className="status-bar">{statusMsg}</div>
    </aside>
  );
}

