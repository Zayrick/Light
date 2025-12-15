import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Toggle } from "@ark-ui/react/toggle";
import { Monitor, RefreshCw, Zap, ArrowRight, BookOpen, Puzzle, Server, ServerOff, ArrowLeft } from "lucide-react";
import clsx from "clsx";
import { Device, EffectInfo } from "../../types";
import { Button } from "../../components/ui/Button";
import { Card } from "../../components/ui/Card";
import { BLOG_POSTS, PLUGINS } from "../../data/mockData";
import styles from "./HomePage.module.css";

type NewsPanelState = "auto" | "hidden" | "compact-news";

const SIDEBAR_BREAKPOINT = 1000;

interface HomePageProps {
  devices: Device[];
  effects: EffectInfo[];
  isScanning: boolean;
  onScan: () => void;
  onNavigate: (devicePort: string) => void;
}

export function HomePage({
  devices,
  effects,
  isScanning,
  onScan,
  onNavigate,
}: HomePageProps) {
  const [panelState, setPanelState] = useState<NewsPanelState>("auto");
  const [isNarrow, setIsNarrow] = useState(false);

  useEffect(() => {
    const updateViewport = () => {
      if (typeof window === "undefined") return;
      setIsNarrow(window.innerWidth < SIDEBAR_BREAKPOINT);
    };

    updateViewport();
    window.addEventListener("resize", updateViewport);

    return () => {
      window.removeEventListener("resize", updateViewport);
    };
  }, []);

  useEffect(() => {
    if (!isNarrow && panelState === "compact-news") {
      setPanelState("auto");
    }
  }, [isNarrow, panelState]);

  const shouldShowSidebar = !isNarrow && panelState === "auto";
  const showCompactNews = isNarrow && panelState === "compact-news";
  const togglePressed = panelState !== "auto";

  const handleTogglePress = (pressed: boolean) => {
    if (isNarrow) {
      setPanelState(pressed ? "compact-news" : "auto");
      return;
    }
    setPanelState(pressed ? "hidden" : "auto");
  };

  const handleBackToDevices = () => {
    setPanelState("auto");
  };

  const newsAndPlugins = (
    <>
      {/* Blogs */}
      <section>
        <div className="section-header">
          <h2 className="section-title">Latest News</h2>
          <Button variant="secondary" style={{ padding: 6 }}>
            <BookOpen size={14} />
          </Button>
        </div>
        <Card style={{ padding: "16px" }}>
          <div className={styles.blogPost}>
            {BLOG_POSTS.map((blog) => (
              <div key={blog.id} className="blog-card">
                <div className="blog-image" />
                <div>
                  <div className={styles.blogMeta}>
                    <span className="blog-date">{blog.date}</span>
                  </div>
                  <h4 className={`blog-title ${styles.blogTitle}`}>{blog.title}</h4>
                  <p className="blog-summary">{blog.summary}</p>
                </div>
              </div>
            ))}
          </div>
        </Card>
      </section>

      {/* Plugins */}
      <section>
        <div className="section-header">
          <h2 className="section-title">Recommended Plugins</h2>
          <Button variant="secondary" style={{ padding: 6 }}>
            <Puzzle size={14} />
          </Button>
        </div>
        <Card style={{ padding: 0 }}>
          <div style={{ display: "flex", flexDirection: "column" }}>
            {PLUGINS.map((plugin) => (
              <div key={plugin.id} className={`plugin-item ${styles.pluginItem}`}>
                <div className="plugin-icon">
                  {plugin.icon}
                </div>
                <div className="plugin-info">
                  <h4 className="plugin-name">{plugin.name}</h4>
                  <p className="plugin-desc">{plugin.description}</p>
                </div>
                <Button variant="secondary" className={styles.pluginButton}>
                  Get
                </Button>
              </div>
            ))}
          </div>
        </Card>
      </section>
    </>
  );

  return (
    <div className={styles.dashboardContainer}>
      {/* Main Content Area (Devices OR Compact News) */}
      <AnimatePresence mode="wait" initial={false}>
        {!showCompactNews ? (
          <motion.div
            key="dashboard-main"
            className={styles.dashboardMain}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
          >
            <div className="section-header">
              <div>
                <h2 className="section-title">My Devices</h2>
                <p className="page-subtitle">
                  Manage your connected lighting devices
                </p>
              </div>
              <div className={styles.headerActions}>
                <Button onClick={onScan} disabled={isScanning} style={{ height: 36 }}>
                  <RefreshCw size={16} className={clsx(isScanning && "animate-spin")} />
                  Scan Devices
                </Button>
                <Toggle.Root
                  className={styles.newsToggle}
                  pressed={togglePressed}
                  onPressedChange={handleTogglePress}
                  aria-label="Toggle news panel visibility"
                >
                  {panelState === "hidden" ? (
                    <ServerOff size={16} />
                  ) : (
                    <Server size={16} />
                  )}
                </Toggle.Root>
              </div>
            </div>

            {devices.length === 0 && !isScanning ? (
              <div className={styles.emptyStateContainer}>
                <Monitor size={48} className={styles.emptyStateIcon} />
                <p>No devices connected</p>
                <Button variant="secondary" style={{ marginTop: 16 }} onClick={onScan}>
                  Try Again
                </Button>
              </div>
            ) : (
              <div className="simplified-device-list">
                {devices.map((device) => {
                  const activeEffectId =
                    device.mode.effective_effect_id ??
                    device.outputs
                      .map((o) => o.mode.effective_effect_id)
                      .find((id): id is string => Boolean(id)) ??
                    device.outputs
                      .flatMap((o) => o.segments)
                      .map((s) => s.mode.effective_effect_id)
                      .find((id): id is string => Boolean(id));

                  return (
                    <Card
                      key={device.port}
                      className="simplified-device-card"
                      hoverable
                      onClick={() => onNavigate(device.port)}
                    >
                      <div className="simplified-device-info">
                        <div className="device-icon">
                          <Zap size={20} />
                        </div>
                        <div>
                          <h3 className={styles.deviceInfoTitle}>{device.model}</h3>
                          <div className={styles.deviceInfoSubtitle}>
                            <span>{device.port}</span>
                            {activeEffectId && (
                              <>
                                <span>•</span>
                                <span>
                                  {effects.find((e) => e.id === activeEffectId)?.name ||
                                    "Unknown Effect"}
                                </span>
                              </>
                            )}
                          </div>
                        </div>
                      </div>

                      <div className="simplified-device-footer">
                        {activeEffectId ? (
                          <div className={styles.deviceActiveStatus}>
                            <div className="device-status-dot" />
                            <span className={styles.deviceActiveText}>Active</span>
                          </div>
                        ) : (
                          <div /> /* Spacer */
                        )}
                        <Button
                          variant="secondary"
                          style={{ padding: "6px 10px" }}
                          onClick={(e) => {
                            e.stopPropagation();
                            onNavigate(device.port);
                          }}
                        >
                          <ArrowRight size={16} />
                        </Button>
                      </div>
                    </Card>
                  );
                })}
              </div>
            )}
          </motion.div>
        ) : (
          <motion.div
            key="compact-news"
            className={styles.compactNewsPage}
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 12 }}
            transition={{ duration: 0.35, ease: [0.25, 0.1, 0.25, 1] }}
          >
            <div className={styles.compactNewsHeader}>
              <Button
                variant="secondary"
                className={styles.compactBackButton}
                onClick={handleBackToDevices}
              >
                <ArrowLeft size={14} />
                <span>Back</span>
              </Button>
              <div>
                <h2 className="section-title">News & Plugins</h2>
                <p className="page-subtitle">Latest updates and add-ons</p>
              </div>
            </div>
            <div className={styles.compactNewsContent}>
              {newsAndPlugins}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Sidebar: Blogs & Plugins (Desktop Only) */}
      <AnimatePresence>
        {shouldShowSidebar && (
          <motion.div
            key="news-sidebar"
            initial={{ width: 0, opacity: 0 }}
            animate={{ width: 350, opacity: 1 }}
            exit={{ width: 0, opacity: 0 }}
            transition={{ duration: 0.35, ease: [0.25, 0.1, 0.25, 1] }}
            style={{ overflow: "hidden", display: "flex", flexDirection: "column" }}
          >
            <div className={styles.dashboardSidebar} style={{ width: 350, minWidth: 350, paddingRight: 8 }}>
              {newsAndPlugins}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
