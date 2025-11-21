import { LayoutDashboard, Settings, Monitor, RefreshCw, Zap } from "lucide-react";
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import clsx from "clsx";
import { TitleBar } from "./components/TitleBar";
import "./styles/theme.css";
import "./styles/layout.css";

interface Device {
  port: string;
  model: string;
  id: string;
}

interface EffectInfo {
  id: string;
  name: string;
}

export default function App() {
  const [activeTab, setActiveTab] = useState("devices");
  const [devices, setDevices] = useState<Device[]>([]);
  const [effects, setEffects] = useState<EffectInfo[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [statusMsg, setStatusMsg] = useState("Ready");

  useEffect(() => {
    invoke<EffectInfo[]>("get_effects")
      .then(setEffects)
      .catch((err) => console.error("Failed to fetch effects:", err));
    
    // Initial scan
    scanDevices();
  }, []);

  async function scanDevices() {
    setIsScanning(true);
    setStatusMsg("Scanning devices...");
    setDevices([]);
    try {
      const foundDevices = await invoke<Device[]>("scan_devices");
      setDevices(foundDevices);
      setStatusMsg(
        foundDevices.length > 0
          ? `Found ${foundDevices.length} device(s)`
          : "No devices found"
      );
    } catch (error) {
      console.error(error);
      setStatusMsg("Error scanning devices");
    } finally {
      setIsScanning(false);
    }
  }

  async function setEffect(port: string, effectId: string) {
    try {
      await invoke("set_effect", { port, effectId });
    } catch (error) {
      console.error(error);
      setStatusMsg(`Failed to set effect: ${error}`);
    }
  }

  return (
    <div className="app-layout">
      <TitleBar />
      
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-content">
          <div className="nav-item active">
            <Monitor size={18} />
            <span>Devices</span>
          </div>
          <div className="nav-item" style={{ opacity: 0.5, cursor: 'not-allowed' }}>
            <Settings size={18} />
            <span>Settings</span>
          </div>
        </div>
        
        <div className="status-bar">
          {statusMsg}
        </div>
      </aside>

      {/* Main Content */}
      <main className="main-content">
        <div className="scroll-container">
          <header className="page-header">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div>
                <h1 className="page-title">Connected Devices</h1>
                <p className="page-subtitle">Manage your lighting devices and effects</p>
              </div>
              <button 
                className={clsx("btn btn-primary", isScanning && "opacity-70")} 
                onClick={scanDevices}
                disabled={isScanning}
              >
                <RefreshCw size={16} className={clsx(isScanning && "animate-spin")} />
                Scan Devices
              </button>
            </div>
          </header>

          {devices.length === 0 && !isScanning ? (
            <div style={{ 
              display: 'flex', 
              flexDirection: 'column', 
              alignItems: 'center', 
              justifyContent: 'center', 
              height: '50%',
              color: 'var(--text-secondary)'
            }}>
              <Monitor size={48} style={{ marginBottom: 16, opacity: 0.3 }} />
              <p>No devices connected</p>
              <button className="btn btn-secondary" style={{ marginTop: 16 }} onClick={scanDevices}>
                Try Again
              </button>
            </div>
          ) : (
            <div className="devices-grid">
              {devices.map((dev, idx) => (
                <div key={idx} className="device-card">
                  <div className="device-header">
                    <div className="device-info">
                      <h3>{dev.model}</h3>
                      <p>{dev.id}</p>
                      <p style={{ fontSize: 10, opacity: 0.7 }}>{dev.port}</p>
                    </div>
                    <div className="device-icon">
                      <Zap size={20} />
                    </div>
                  </div>
                  
                  <div style={{ margin: '12px 0', fontSize: 12, fontWeight: 600, color: 'var(--text-secondary)' }}>
                    Quick Effects
                  </div>
                  
                  <div className="device-actions">
                    {effects.map((effect) => (
                      <button
                        key={effect.id}
                        className="btn btn-secondary"
                        style={{ fontSize: 11, padding: '4px 8px' }}
                        onClick={() => setEffect(dev.port, effect.id)}
                      >
                        {effect.name}
                      </button>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
