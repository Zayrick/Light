import { Monitor, RefreshCw, Zap, ArrowRight, BookOpen, Puzzle } from "lucide-react";
import clsx from "clsx";
import { Device, EffectInfo } from "../../types";
import { Button } from "../../components/ui/Button";
import { Card } from "../../components/ui/Card";
import { BLOG_POSTS, PLUGINS } from "../../data/mockData";
import styles from "./HomePage.module.css";

interface HomePageProps {
  devices: Device[];
  effects: EffectInfo[];
  isScanning: boolean;
  onScan: () => void;
  onNavigate: (deviceId: string) => void;
}

export function HomePage({
  devices,
  effects,
  isScanning,
  onScan,
  onNavigate,
}: HomePageProps) {
  return (
    <div className="dashboard-container">
      {/* Main Area: Devices */}
      <div className="dashboard-main">
        <div className="section-header">
          <div>
            <h2 className="section-title">My Devices</h2>
            <p className="page-subtitle">
              Manage your connected lighting devices
            </p>
          </div>
          <Button onClick={onScan} disabled={isScanning}>
            <RefreshCw size={16} className={clsx(isScanning && "animate-spin")} />
            Scan Devices
          </Button>
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
            {devices.map((device) => (
              <Card
                key={device.id}
                className="simplified-device-card"
                hoverable
                onClick={() => onNavigate(device.id)}
              >
                <div className="simplified-device-info">
                  <div className="device-icon">
                    <Zap size={20} />
                  </div>
                  <div>
                    <h3 className={styles.deviceInfoTitle}>
                      {device.model}
                    </h3>
                    <div className={styles.deviceInfoSubtitle}>
                      <span>{device.port}</span>
                      {device.current_effect_id && (
                        <>
                          <span>•</span>
                          <span>
                            {effects.find(e => e.id === device.current_effect_id)?.name || "Unknown Effect"}
                          </span>
                        </>
                      )}
                    </div>
                  </div>
                </div>
                
                <div className="simplified-device-footer">
                  {device.current_effect_id ? (
                    <div className={styles.deviceActiveStatus}>
                      <div className="device-status-dot" />
                      <span className={styles.deviceActiveText}>
                        Active
                      </span>
                    </div>
                  ) : (
                    <div /> /* Spacer */
                  )}
                  <Button
                    variant="secondary"
                    style={{ padding: "6px 10px" }}
                    onClick={(e) => {
                      e.stopPropagation();
                      onNavigate(device.id);
                    }}
                  >
                     <ArrowRight size={16} />
                  </Button>
                </div>
              </Card>
            ))}
          </div>
        )}
      </div>

      {/* Sidebar: Blogs & Plugins */}
      <div className="dashboard-sidebar">
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
      </div>
    </div>
  );
}
