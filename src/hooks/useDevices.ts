import { useState, useEffect, useCallback } from "react";
import type { Device, SelectedScope } from "../types";
import { api } from "../services/api";
import { logger } from "../services/logger";
import { normalizeSelectedScope } from "../utils/scope";

export function useDevices() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedScope, setSelectedScope] = useState<SelectedScope | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [statusMsg, setStatusMsg] = useState("Ready");

  const selectScope = useCallback(
    (scope: SelectedScope | null) => {
      setSelectedScope(scope ? normalizeSelectedScope(scope, devices) : null);
    },
    [devices]
  );

  const scanDevices = useCallback(async () => {
    setIsScanning(true);
    setStatusMsg("Scanning devices...");
    setDevices([]);
    try {
      const foundDevices = await api.scanDevices();
      setDevices(foundDevices);
      if (foundDevices.length > 0) {
        // Preserve selection if still exists, otherwise select first device scope
        setSelectedScope((prev) => {
          const stillExists = prev && foundDevices.some((d) => d.port === prev.port);
          const next = stillExists ? prev! : { port: foundDevices[0].port };
          return normalizeSelectedScope(next, foundDevices);
        });
      } else {
        setSelectedScope(null);
      }
      setStatusMsg(
        foundDevices.length > 0
          ? `Found ${foundDevices.length} device(s)`
          : "No devices found"
      );
    } catch (error) {
      logger.error("devices.scan_failed", {}, error);
      setStatusMsg("Error scanning devices");
    } finally {
      setIsScanning(false);
    }
  }, []);

  const refreshDevices = useCallback(async () => {
    try {
      const current = await api.getDevices();
      setDevices(current);
      if (current.length > 0) {
        setSelectedScope((prev) => {
          const stillExists = prev && current.some((d) => d.port === prev.port);
          const next = stillExists ? prev! : { port: current[0].port };
          return normalizeSelectedScope(next, current);
        });
      } else {
        setSelectedScope(null);
      }
    } catch (error) {
      logger.error("devices.refresh_failed", {}, error);
    }
  }, []);

  // Initial scan
  useEffect(() => {
    scanDevices();
  }, [scanDevices]);

  return {
    devices,
    selectedScope,
    selectScope,
    isScanning,
    statusMsg,
    scanDevices,
    refreshDevices,
  };
}

