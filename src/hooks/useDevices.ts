import { useState, useEffect, useCallback } from "react";
import { Device } from "../types";
import { api } from "../services/api";
import { logger } from "../services/logger";

export function useDevices() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<Device | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [statusMsg, setStatusMsg] = useState("Ready");

  const updateDeviceEffect = useCallback((port: string, effectId: string | null) => {
    setDevices((prev) =>
      prev.map((dev) =>
        dev.port === port
          ? {
              ...dev,
              current_effect_id: effectId ?? undefined,
              current_effect_params: undefined,
            }
          : dev
      )
    );

    setSelectedDevice((prev) =>
      prev && prev.port === port
        ? {
            ...prev,
            current_effect_id: effectId ?? undefined,
            current_effect_params: undefined,
          }
        : prev
    );
  }, []);

  const updateDeviceParams = useCallback(
    (port: string, params: Record<string, number | boolean>) => {
      setDevices((prev) =>
        prev.map((dev) =>
          dev.port === port
            ? {
                ...dev,
                current_effect_params: {
                  ...(dev.current_effect_params ?? {}),
                  ...params,
                },
              }
            : dev
        )
      );

      setSelectedDevice((prev) =>
        prev && prev.port === port
          ? {
              ...prev,
              current_effect_params: {
                ...(prev.current_effect_params ?? {}),
                ...params,
              },
            }
          : prev
      );
    },
    []
  );

  const updateDeviceBrightness = useCallback((port: string, brightness: number) => {
    setDevices((prev) =>
      prev.map((dev) => (dev.port === port ? { ...dev, brightness } : dev))
    );

    setSelectedDevice((prev) =>
      prev && prev.port === port ? { ...prev, brightness } : prev
    );
  }, []);

  const scanDevices = useCallback(async () => {
    setIsScanning(true);
    setStatusMsg("Scanning devices...");
    setDevices([]);
    try {
      const foundDevices = await api.scanDevices();
      setDevices(foundDevices);
      if (foundDevices.length > 0) {
        // Preserve selection if still exists, otherwise select first
        setSelectedDevice((prev) => {
          const stillExists = foundDevices.find((d) => d.id === prev?.id);
          return stillExists || foundDevices[0];
        });
      } else {
        setSelectedDevice(null);
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

  // Initial scan
  useEffect(() => {
    scanDevices();
  }, [scanDevices]);

  return {
    devices,
    selectedDevice,
    setSelectedDevice,
    isScanning,
    statusMsg,
    scanDevices,
    updateDeviceEffect,
    updateDeviceParams,
    updateDeviceBrightness,
  };
}

