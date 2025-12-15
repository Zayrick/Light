import { Home, Settings, Zap } from "lucide-react";
import clsx from "clsx";
import { motion, AnimatePresence } from "framer-motion";
import { useCallback, memo } from "react";
import { Device } from "../../types";
import { SidebarLayoutTree } from "./SidebarLayoutTree";
import { HIGHLIGHT_TRANSITION, NAV_TRANSITION } from "./constants";

const ActiveHighlight = memo(() => (
  <motion.div
    layoutId="active-nav"
    className="active-highlight"
    transition={HIGHLIGHT_TRANSITION}
  />
));

ActiveHighlight.displayName = "ActiveHighlight";

interface SidebarProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
  devices: Device[];
  selectedDevice: Device | null;
  setSelectedDevice: (device: Device | null) => void;
  selectedLayoutId: string | null;
  setSelectedLayoutId: (id: string | null) => void;
}

export function Sidebar({
  activeTab,
  setActiveTab,
  devices,
  selectedDevice,
  setSelectedDevice,
  selectedLayoutId,
  setSelectedLayoutId,
}: SidebarProps) {
  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    e.currentTarget.style.setProperty("--mouse-x", `${x}px`);
    e.currentTarget.style.setProperty("--mouse-y", `${y}px`);
    e.currentTarget.style.setProperty("--spotlight-opacity", "1");
  }, []);

  const handleMouseLeave = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    e.currentTarget.style.setProperty("--spotlight-opacity", "0");
  }, []);

  const handleLayoutSelect = useCallback((id: string) => {
    setSelectedLayoutId(id);
    setSelectedDevice(null); // Clear device selection
    setActiveTab("layout-preview");
  }, [setSelectedLayoutId, setSelectedDevice, setActiveTab]);

  const handleDeviceSelect = useCallback((device: Device) => {
    setSelectedDevice(device);
    setSelectedLayoutId(null); // Clear layout selection
    setActiveTab("device-detail");
  }, [setSelectedDevice, setSelectedLayoutId, setActiveTab]);

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
              {activeTab === "home" && <ActiveHighlight />}
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
                onClick={() => handleDeviceSelect(device)}
                onMouseMove={handleMouseMove}
                onMouseLeave={handleMouseLeave}
              >
                <AnimatePresence>
                  {activeTab === "device-detail" &&
                    selectedDevice?.id === device.id && <ActiveHighlight />}
                </AnimatePresence>
                <Zap size={18} className="device-list-icon" />
                <div className="device-list-info">
                  <div className="device-list-item-name">{device.model}</div>
                  <div className="device-list-item-port">{device.port}</div>
                </div>
              </div>
            ))}

            <SidebarLayoutTree
              activeTab={activeTab}
              selectedLayoutId={selectedLayoutId}
              onSelect={handleLayoutSelect}
              onMouseMove={handleMouseMove}
              onMouseLeave={handleMouseLeave}
            />
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
            {activeTab === "settings" && <ActiveHighlight />}
            <Settings size={18} />
            <span>Settings</span>
          </motion.div>
        </div>
      </div>
    </aside>
  );
}

