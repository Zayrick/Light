import { Monitor, RefreshCw, Zap, Music, Cast, Bell, BookOpen, Puzzle, ArrowRight } from "lucide-react";
import clsx from "clsx";
import { Device, EffectInfo } from "../../types";
import { Button } from "../../components/ui/Button";
import { Card } from "../../components/ui/Card";

interface HomePageProps {
  devices: Device[];
  effects: EffectInfo[];
  isScanning: boolean;
  onScan: () => void;
  onSetEffect: (port: string, effectId: string) => Promise<void>;
  onNavigate: (deviceId: string) => void;
}

const BLOG_POSTS = [
  {
    id: 1,
    title: "Getting Started with Light",
    summary: "Learn how to set up your first device and configure basic effects.",
    date: "Oct 24, 2023",
  },
  {
    id: 2,
    title: "Advanced Effect Creator",
    summary: "Deep dive into creating custom matrix animations.",
    date: "Nov 02, 2023",
  },
  {
    id: 3,
    title: "Community Showcase",
    summary: "Check out the most popular setups from our community this week.",
    date: "Nov 15, 2023",
  },
];

const PLUGINS = [
  {
    id: 1,
    name: "Audio Visualizer",
    description: "Sync lights with music beat",
    icon: <Music size={18} />,
  },
  {
    id: 2,
    name: "Screen Mirror",
    description: "Extend screen colors to lights",
    icon: <Cast size={18} />,
  },
  {
    id: 3,
    name: "Notifications",
    description: "Flash on new messages",
    icon: <Bell size={18} />,
  },
];

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
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              height: "300px",
              color: "var(--text-secondary)",
              backgroundColor: "var(--bg-card)",
              borderRadius: "var(--radius-m)",
              border: "1px solid var(--border-subtle)",
            }}
          >
            <Monitor size={48} style={{ marginBottom: 16, opacity: 0.3 }} />
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
                    <h3 style={{ margin: "0 0 4px 0", fontSize: 15, fontWeight: 600 }}>
                      {device.model}
                    </h3>
                    <div style={{ display: "flex", gap: 12, fontSize: 12, color: "var(--text-secondary)" }}>
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
                    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                      <div className="device-status-dot" />
                      <span style={{ fontSize: 12, fontWeight: 500, color: "var(--text-secondary)" }}>
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
            <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
              {BLOG_POSTS.map((blog) => (
                <div key={blog.id} className="blog-card">
                  <div className="blog-image" />
                  <div>
                    <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                      <span className="blog-date">{blog.date}</span>
                    </div>
                    <h4 className="blog-title" style={{ marginBottom: 4 }}>{blog.title}</h4>
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
                <div key={plugin.id} className="plugin-item" style={{ padding: "16px" }}>
                  <div className="plugin-icon">
                    {plugin.icon}
                  </div>
                  <div className="plugin-info">
                    <h4 className="plugin-name">{plugin.name}</h4>
                    <p className="plugin-desc">{plugin.description}</p>
                  </div>
                  <Button variant="secondary" style={{ padding: "4px 12px", fontSize: 11, height: 28 }}>
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
