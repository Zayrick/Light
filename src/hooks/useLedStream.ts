import { useEffect, useMemo, useState, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { LedColor as LedColorType } from "../types";
import { logger } from "../services/logger";

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
      logger.error("ledStream.listener.init_failed", {}, err);
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

// Throttle interval in ms - limits React state updates to ~30fps
// The backend can still send updates faster, but we only re-render at this rate
const THROTTLE_MS = 33;

// Convenience hook for components with built-in throttling
export function useLedColors(port: string, length: number) {
  const [colors, setColors] = useState<LedColor[] | null>(() => getLatestColors(port));
  
  // Refs for throttling
  const pendingColorsRef = useRef<LedColor[] | null>(null);
  const throttleTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastUpdateRef = useRef<number>(0);

  // Throttled setter
  const throttledSetColors = useCallback((newColors: LedColor[]) => {
    pendingColorsRef.current = newColors;
    
    const now = Date.now();
    const timeSinceLastUpdate = now - lastUpdateRef.current;
    
    // If enough time has passed, update immediately
    if (timeSinceLastUpdate >= THROTTLE_MS) {
      lastUpdateRef.current = now;
      setColors(newColors);
      pendingColorsRef.current = null;
      return;
    }
    
    // Otherwise, schedule an update if not already scheduled
    if (!throttleTimeoutRef.current) {
      throttleTimeoutRef.current = setTimeout(() => {
        if (pendingColorsRef.current) {
          lastUpdateRef.current = Date.now();
          setColors(pendingColorsRef.current);
          pendingColorsRef.current = null;
        }
        throttleTimeoutRef.current = null;
      }, THROTTLE_MS - timeSinceLastUpdate);
    }
  }, []);

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
        if (mounted) {
          throttledSetColors(next);
        }
      });
    };

    setup();

    return () => {
      mounted = false;
      if (unsubLocal) unsubLocal();
      // Clear any pending throttle timeout
      if (throttleTimeoutRef.current) {
        clearTimeout(throttleTimeoutRef.current);
        throttleTimeoutRef.current = null;
      }
    };
  }, [port, throttledSetColors]);

  return { colors: displayColors, isDefault: !colors || colors.length === 0 };
}
