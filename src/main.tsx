import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { initLogging } from "./services/logger";

async function bootstrap() {
  await initLogging();

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
