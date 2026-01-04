import { Home, Settings } from "lucide-react";
import clsx from "clsx";
import { motion, LayoutGroup } from "framer-motion";
import { useCallback } from "react";
import type { Device, SelectedScope } from "../../types";
import { SidebarDeviceTree } from "./SidebarDeviceTree";
import { HIGHLIGHT_TRANSITION, NAV_TRANSITION } from "../../motion/transitions";

const ActiveHighlight = () => (
  <motion.div
    layoutId="sidebar-active-highlight"
    className="active-highlight"
    transition={HIGHLIGHT_TRANSITION}
    style={{ zIndex: -1 }}
  />
);

interface SidebarProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
  devices: Device[];
  selectedScope: SelectedScope | null;
  selectScope: (scope: SelectedScope | null) => void;
}

export function Sidebar({
  activeTab,
  setActiveTab,
  devices,
  selectedScope,
  selectScope,
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

  const handleScopeSelect = useCallback((scope: SelectedScope) => {
    selectScope(scope);
    setActiveTab("device-detail");
  }, [selectScope, setActiveTab]);

  return (
    <aside className="sidebar">
      <LayoutGroup id="sidebar-nav">
        <div className="sidebar-content">
          <div className="nav-group nav-group-main">
            <div>
              <motion.div
                layout
                className={clsx("nav-item", activeTab === "home" && "active")}
                onClick={() => setActiveTab("home")}
                onMouseMove={handleMouseMove}
                onMouseLeave={handleMouseLeave}
                animate={{ fontWeight: activeTab === "home" ? 600 : 400 }}
                transition={NAV_TRANSITION}
              >
                {activeTab === "home" && <ActiveHighlight />}
                <Home size={18} />
                <span>Home</span>
              </motion.div>
              <div className="nav-divider" />
            </div>
            <div className="device-list">
              <SidebarDeviceTree
                activeTab={activeTab}
                devices={devices}
                selectedScope={selectedScope}
                onSelectScope={handleScopeSelect}
                onMouseMove={handleMouseMove}
                onMouseLeave={handleMouseLeave}
              />
            </div>
          </div>
          <div className="nav-group nav-group-settings">
            <motion.div
              layout
              className={clsx("nav-item", activeTab === "settings" && "active")}
              onClick={() => setActiveTab("settings")}
              onMouseMove={handleMouseMove}
              onMouseLeave={handleMouseLeave}
              animate={{ fontWeight: activeTab === "settings" ? 600 : 400 }}
              transition={NAV_TRANSITION}
            >
              {activeTab === "settings" && <ActiveHighlight />}
              <Settings size={18} />
              <span>Settings</span>
            </motion.div>
          </div>
        </div>
      </LayoutGroup>
    </aside>
  );
}

