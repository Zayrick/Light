export const APP_SETTINGS_CHANGED_EVENT = "light:app-settings-changed" as const;

const MINIMIZE_TO_TRAY_KEY = "light.settings.minimizeToTray";

export function readMinimizeToTraySetting(): boolean {
  try {
    const raw = localStorage.getItem(MINIMIZE_TO_TRAY_KEY);
    if (raw === null) return false;
    return raw === "true";
  } catch {
    return false;
  }
}

export function writeMinimizeToTraySetting(enabled: boolean): void {
  try {
    localStorage.setItem(MINIMIZE_TO_TRAY_KEY, String(enabled));
  } finally {
    // 同窗口内 localStorage 变更不会触发 storage 事件，
    // 用自定义事件通知其他组件同步。
    window.dispatchEvent(
      new CustomEvent(APP_SETTINGS_CHANGED_EVENT, {
        detail: { key: "minimizeToTray", value: enabled },
      }),
    );
  }
}

export function onAppSettingsChanged(
  handler: (detail: { key: string; value: unknown }) => void,
): () => void {
  const listener = (evt: Event) => {
    const custom = evt as CustomEvent;
    handler(custom.detail as { key: string; value: unknown });
  };

  window.addEventListener(APP_SETTINGS_CHANGED_EVENT, listener);
  return () => window.removeEventListener(APP_SETTINGS_CHANGED_EVENT, listener);
}
