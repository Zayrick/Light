import { useState, useEffect, useCallback } from "react";
import { Device } from "../types";
import { api } from "../services/api";
import { logger } from "../services/logger";

export interface SelectedScope {
  port: string;
  outputId?: string;
  segmentId?: string;
}

function normalizeScope(scope: SelectedScope, devices: Device[]): SelectedScope {
  const device = devices.find((d) => d.port === scope.port);
  if (!device) return scope;

  // If the device has a single output, always treat "device scope" as selecting the default output.
  if (!scope.outputId && device.outputs.length === 1) {
    return { port: scope.port, outputId: device.outputs[0].id };
  }

  if (scope.outputId) {
    const out = device.outputs.find((o) => o.id === scope.outputId);
    if (!out) {
      // Output no longer exists. Fall back to a stable scope.
      return device.outputs.length === 1
        ? { port: scope.port, outputId: device.outputs[0].id }
        : { port: scope.port };
    }

    if (scope.segmentId) {
      const segExists = out.segments.some((s) => s.id === scope.segmentId);
      if (!segExists) return { port: scope.port, outputId: scope.outputId };
    }
  }

  return scope;
}

export function useDevices() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedScope, setSelectedScope] = useState<SelectedScope | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [statusMsg, setStatusMsg] = useState("Ready");

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
          return normalizeScope(next, foundDevices);
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
          return normalizeScope(next, current);
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
    setSelectedScope,
    isScanning,
    statusMsg,
    scanDevices,
    refreshDevices,
  };
}

