import {
  attachConsole,
  debug as pluginDebug,
  error as pluginError,
  info as pluginInfo,
  trace as pluginTrace,
  warn as pluginWarn,
} from "@tauri-apps/plugin-log";

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";
export type LogContext = Record<string, unknown>;

function createSessionId(): string {
  if (globalThis.crypto?.randomUUID) return globalThis.crypto.randomUUID();
  // Fallback: not cryptographically strong, but stable enough for log correlation.
  return `session_${Date.now()}_${Math.random().toString(16).slice(2)}`;
}

const SESSION_ID = createSessionId();

function serializeError(err: unknown): unknown {
  if (!err) return err;
  if (err instanceof Error) {
    return {
      name: err.name,
      message: err.message,
      stack: err.stack,
      cause: serializeError((err as any).cause),
    };
  }

  // DOMException / other thrown types
  if (typeof err === "object") {
    try {
      return JSON.parse(JSON.stringify(err));
    } catch {
      return String(err);
    }
  }

  return err;
}

function safeJsonStringify(value: unknown): string {
  try {
    return JSON.stringify(value, (_k, v) => {
      if (v instanceof Error) return serializeError(v);
      if (typeof v === "bigint") return v.toString();
      return v;
    });
  } catch {
    return JSON.stringify({ msg: "Failed to serialize log payload" });
  }
}

function makeMessage(message: string, ctx?: LogContext, err?: unknown): string {
  const payload: Record<string, unknown> = {
    msg: message,
    ctx: {
      sessionId: SESSION_ID,
      ...ctx,
    },
  };

  if (err !== undefined) payload.err = serializeError(err);

  return safeJsonStringify(payload);
}

export const logger = {
  trace: (message: string, ctx?: LogContext) => pluginTrace(makeMessage(message, ctx)),
  debug: (message: string, ctx?: LogContext) => pluginDebug(makeMessage(message, ctx)),
  info: (message: string, ctx?: LogContext) => pluginInfo(makeMessage(message, ctx)),
  warn: (message: string, ctx?: LogContext) => pluginWarn(makeMessage(message, ctx)),
  error: (message: string, ctx?: LogContext, err?: unknown) =>
    pluginError(makeMessage(message, ctx, err)),
};

let detachConsole: (() => void) | undefined;

export async function initLogging(): Promise<void> {
  // In dev, mirror plugin logs into the browser console for convenience.
  if (import.meta.env.DEV) {
    try {
      detachConsole = await attachConsole();
    } catch (err) {
      // If this fails we still want the app to work.
      console.warn("[log] attachConsole failed", err);
    }
  }

  window.addEventListener("error", (event) => {
    logger.error(
      "window.error",
      {
        message: event.message,
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
      },
      event.error,
    );
  });

  window.addEventListener("unhandledrejection", (event) => {
    logger.error("window.unhandledrejection", {}, event.reason);
  });

  logger.info("frontend.start", {
    mode: import.meta.env.MODE,
    ua: navigator.userAgent,
  });
}

export function teardownLogging(): void {
  try {
    detachConsole?.();
  } finally {
    detachConsole = undefined;
  }
}

export function getLogSessionId(): string {
  return SESSION_ID;
}

