import { useCallback, useEffect, useState } from "react";
import { api } from "../services/api";
import {
  onAppSettingsChanged,
  readMinimizeToTraySetting,
  writeMinimizeToTraySetting,
} from "../utils/appSettings";
import { logger } from "../services/logger";

export function useMinimizeToTray() {
  const [enabled, setEnabled] = useState<boolean>(() => readMinimizeToTraySetting());

  // 同步来自其它组件的修改
  useEffect(() => {
    return onAppSettingsChanged((detail) => {
      if (detail.key === "minimizeToTray" && typeof detail.value === "boolean") {
        setEnabled(detail.value);
      }
    });
  }, []);

  // 首次挂载时，确保后端知道当前偏好
  useEffect(() => {
    api.setMinimizeToTray(enabled).catch((err) => {
      logger.error("settings.minimize_to_tray.sync_failed", { enabled }, err);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const setMinimizeToTray = useCallback(async (next: boolean) => {
    setEnabled(next);
    writeMinimizeToTraySetting(next);

    try {
      await api.setMinimizeToTray(next);
    } catch (err) {
      logger.error("settings.minimize_to_tray.set_failed", { enabled: next }, err);
      // best-effort: 不回滚 UI，让用户在 UI 上保持期望值；
      // 后续可在 Settings 页面补充更显性的错误提示。
    }
  }, []);

  return { minimizeToTray: enabled, setMinimizeToTray };
}
