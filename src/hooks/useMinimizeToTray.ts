import { useCallback, useState } from "react";
import { api } from "../services/api";
import { logger } from "../services/logger";

export function useMinimizeToTray() {
  const [enabled, setEnabled] = useState<boolean>(false);

  const setMinimizeToTray = useCallback(async (next: boolean) => {
    setEnabled(next);
    try {
      await api.setMinimizeToTray(next);
    } catch (err) {
      logger.error("settings.minimize_to_tray.set_failed", { enabled: next }, err);
    }
  }, []);

  return { minimizeToTray: enabled, setMinimizeToTray };
}
