import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface Device {
  port: string;
  model: string;
  id: string;
}

function App() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [statusMsg, setStatusMsg] = useState("");

  async function scanDevices() {
    setIsScanning(true);
    setStatusMsg("Scanning...");
    setDevices([]);
    try {
      const foundDevices = await invoke<Device[]>("scan_devices");
      setDevices(foundDevices);
      setStatusMsg(
        foundDevices.length > 0 
          ? `Found ${foundDevices.length} device(s).` 
          : "No devices found."
      );
    } catch (error) {
      console.error(error);
      setStatusMsg("Error scanning devices.");
    } finally {
      setIsScanning(false);
    }
  }

  async function setRainbow(port: string) {
    try {
      await invoke("set_rainbow", { port });
      setStatusMsg(`Sent Rainbow to ${port}`);
    } catch (error) {
      console.error(error);
      setStatusMsg(`Failed to send Rainbow to ${port}: ${error}`);
    }
  }

  async function turnOff(port: string) {
    try {
      await invoke("turn_off", { port });
      setStatusMsg(`Turned off ${port}`);
    } catch (error) {
      console.error(error);
      setStatusMsg(`Failed to turn off ${port}: ${error}`);
    }
  }

  return (
    <main className="container">
      <h1>Device Scanner</h1>

      <div className="row">
        <button onClick={scanDevices} disabled={isScanning}>
          {isScanning ? "Scanning..." : "Scan Devices"}
        </button>
      </div>

      <p>{statusMsg}</p>

      {devices.length > 0 && (
        <table className="device-table">
          <thead>
            <tr>
              <th>Port</th>
              <th>Model</th>
              <th>ID</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {devices.map((dev, index) => (
              <tr key={index}>
                <td>{dev.port}</td>
                <td>{dev.model}</td>
                <td>{dev.id}</td>
                <td>
                  <div className="action-buttons">
                    <button onClick={() => setRainbow(dev.port)}>彩虹</button>
                    <button onClick={() => turnOff(dev.port)}>关灯</button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      
      <style>{`
        .device-table {
          margin-top: 20px;
          width: 100%;
          border-collapse: collapse;
        }
        .device-table th, .device-table td {
          border: 1px solid #ddd;
          padding: 8px;
          text-align: left;
        }
        .device-table th {
          background-color: #f2f2f2;
          color: #333;
        }
        .action-buttons {
            display: flex;
            gap: 8px;
        }
      `}</style>
    </main>
  );
}

export default App;
