import { useCallback, useEffect, useState } from "react";
import { configManager } from "../services/config";
import { logger } from "../services/logger";

export function useMinimizeToTray() {
  const [enabled, setEnabled] = useState<boolean>(false);

  useEffect(() => {
    let cancelled = false;
    configManager
      .getAppConfig()
      .then((cfg) => {
        if (cancelled) return;
        setEnabled(cfg.minimizeToTray);
      })
      .catch((err) => {
        logger.error("settings.minimize_to_tray.load_failed", {}, err);
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const setMinimizeToTray = useCallback(async (next: boolean) => {
    setEnabled(next);
    try {
      await configManager.setMinimizeToTray(next);
    } catch (err) {
      logger.error("settings.minimize_to_tray.set_failed", { enabled: next }, err);
    }
  }, []);

  return { minimizeToTray: enabled, setMinimizeToTray };
}
