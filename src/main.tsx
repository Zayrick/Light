import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { initLogging } from "./services/logger";
import { api } from "./services/api";
import { readMinimizeToTraySetting } from "./utils/appSettings";

async function bootstrap() {
  await initLogging();

  // 启动阶段尽早同步关键窗口行为偏好到后端（best-effort）
  try {
    await api.setMinimizeToTray(readMinimizeToTraySetting());
  } catch (err) {
    // 日志系统已就绪，但这里不强依赖；失败则保持后端默认行为。
    console.warn("[bootstrap] Failed to sync minimizeToTray", err);
  }

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

bootstrap().catch((err) => {
  console.error("[bootstrap] Failed to start app", err);
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
