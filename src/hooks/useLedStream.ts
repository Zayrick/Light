import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { LedColor as LedColorType } from "../types";

export type LedColor = LedColorType;

type Subscriber = (colors: LedColor[]) => void;

// Singleton event wiring shared across the app lifetime
let isListening = false;
let startListeningPromise: Promise<void> | null = null;

const portToSubscribers = new Map<string, Set<Subscriber>>();
const latestByPort = new Map<string, LedColor[]>();

async function ensureListening() {
  if (isListening) return;
  if (startListeningPromise) return startListeningPromise;

  startListeningPromise = (async () => {
    try {
      await listen<{ port: string; colors: LedColor[] }>(
        "device-led-update",
        (event) => {
          const { port, colors } = event.payload;
          latestByPort.set(port, colors);

          const subs = portToSubscribers.get(port);
          if (!subs || subs.size === 0) return;
          for (const cb of subs) {
            cb(colors);
          }
        }
      );
      isListening = true;
    } catch (err) {
      // Keep silent in production; consumers may retry on demand
      console.error("Failed to initialize LED stream listener:", err);
    } finally {
      startListeningPromise = null;
    }
  })();

  return startListeningPromise;
}

export function subscribeToLedPort(port: string, cb: Subscriber): () => void {
  if (!portToSubscribers.has(port)) {
    portToSubscribers.set(port, new Set());
  }
  portToSubscribers.get(port)!.add(cb);

  return () => {
    const set = portToSubscribers.get(port);
    if (!set) return;
    set.delete(cb);
    if (set.size === 0) {
      portToSubscribers.delete(port);
    }
  };
}

export function getLatestColors(port: string): LedColor[] | null {
  return latestByPort.get(port) ?? null;
}

// Convenience hook for components
export function useLedColors(port: string, length: number) {
  const [colors, setColors] = useState<LedColor[] | null>(() => getLatestColors(port));

  // Keep default gray placeholders when no data yet
  const displayColors = useMemo(() => {
    if (colors && colors.length > 0) return colors;
    return Array.from({ length }, () => ({ r: 128, g: 128, b: 128 }));
  }, [colors, length]);

  useEffect(() => {
    let unsubLocal: (() => void) | null = null;
    let mounted = true;

    const setup = async () => {
      await ensureListening();
      if (!mounted) return;

      // Update from latest cache immediately if available
      const cached = getLatestColors(port);
      if (cached) setColors(cached);

      unsubLocal = subscribeToLedPort(port, (next) => {
        setColors(next);
      });
    };

    setup();

    return () => {
      mounted = false;
      if (unsubLocal) unsubLocal();
    };
  }, [port]);

  return { colors: displayColors, isDefault: !colors || colors.length === 0 };
}


