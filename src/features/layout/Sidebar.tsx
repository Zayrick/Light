import { Home, Settings } from "lucide-react";
import clsx from "clsx";
import { motion } from "framer-motion";
import { useCallback, memo } from "react";
import { Device } from "../../types";
import type { SelectedScope } from "../../hooks/useDevices";
import { SidebarDeviceTree } from "./SidebarDeviceTree";
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
  selectedScope: SelectedScope | null;
  setSelectedScope: (scope: SelectedScope | null) => void;
  selectedLayoutId: string | null;
  setSelectedLayoutId: (id: string | null) => void;
}

export function Sidebar({
  activeTab,
  setActiveTab,
  devices,
  selectedScope,
  setSelectedScope,
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
    setSelectedScope(null); // Clear device selection
    setActiveTab("layout-preview");
  }, [setSelectedLayoutId, setSelectedScope, setActiveTab]);

  const handleScopeSelect = useCallback((scope: SelectedScope) => {
    setSelectedScope(scope);
    setSelectedLayoutId(null); // Clear layout selection
    setActiveTab("device-detail");
  }, [setSelectedScope, setSelectedLayoutId, setActiveTab]);

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
            <SidebarDeviceTree
              activeTab={activeTab}
              devices={devices}
              selectedScope={selectedScope}
              onSelectScope={handleScopeSelect}
              onMouseMove={handleMouseMove}
              onMouseLeave={handleMouseLeave}
            />

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

