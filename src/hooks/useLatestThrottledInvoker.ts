import { useCallback, useEffect, useMemo, useRef } from "react";

export interface LatestThrottledInvokerOptions<T> {
  /**
   * Compare whether two scheduled values are equal.
   * If equal to last-sent value, schedule() becomes a no-op (unless force=true).
   */
  areEqual?: (a: T, b: T) => boolean;
  /**
   * Called when the underlying function throws.
   * Keep it lightweight — this runs on a high-frequency path.
   */
  onError?: (err: unknown) => void;
}

export interface LatestThrottledInvoker<T> {
  /**
   * Schedule a value to be sent.
   * When multiple values are scheduled within the interval, only the latest one is sent.
   */
  schedule: (value: T, options?: { force?: boolean }) => void;
  /** Cancel any pending (not-yet-sent) value. */
  cancel: () => void;
}

/**
 * A React hook that provides a non-blocking, "latest-wins" throttled invoker.
 *
 * Goals:
 * - Avoid blocking UI by never awaiting in event handlers.
 * - Coalesce high-frequency updates (slider drag, color picker drag).
 * - Ensure we never have multiple in-flight invocations piling up.
 *
 * Notes:
 * - If intervalMs is 0, it behaves like a ready/busy gate:
 *   send immediately when idle; if busy, keep only the latest pending value.
 */
export function useLatestThrottledInvoker<T>(
  fn: (value: T) => Promise<unknown> | unknown,
  intervalMs: number,
  options?: LatestThrottledInvokerOptions<T>,
): LatestThrottledInvoker<T> {
  const fnRef = useRef(fn);
  const areEqualRef = useRef(options?.areEqual);
  const onErrorRef = useRef(options?.onError);

  useEffect(() => {
    fnRef.current = fn;
  }, [fn]);

  useEffect(() => {
    areEqualRef.current = options?.areEqual;
    onErrorRef.current = options?.onError;
  }, [options?.areEqual, options?.onError]);

  const timerRef = useRef<number | null>(null);
  const pendingRef = useRef<T | null>(null);
  const inFlightRef = useRef(false);
  const lastSentRef = useRef<T | null>(null);

  const interval = Math.max(0, intervalMs);

  const clearTimer = () => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  const cancel = useCallback(() => {
    clearTimer();
    pendingRef.current = null;
  }, []);

  const flush = useCallback(async () => {
    if (inFlightRef.current) return;

    const next = pendingRef.current;
    if (next === null) return;

    pendingRef.current = null;
    inFlightRef.current = true;

    try {
      await fnRef.current(next);
      lastSentRef.current = next;
    } catch (err) {
      onErrorRef.current?.(err);
    } finally {
      inFlightRef.current = false;

      // If new values arrived while in-flight, schedule another flush.
      if (pendingRef.current !== null && timerRef.current === null) {
        if (interval === 0) {
          // Ready/busy mode: send the latest immediately after the current call finishes.
          // Use a macrotask to avoid deep recursion.
          timerRef.current = window.setTimeout(() => {
            timerRef.current = null;
            void flush();
          }, 0);
        } else {
          timerRef.current = window.setTimeout(() => {
            timerRef.current = null;
            void flush();
          }, interval);
        }
      }
    }
  }, [interval]);

  const schedule = useCallback(
    (value: T, scheduleOptions?: { force?: boolean }) => {
      const isEqual = areEqualRef.current;
      const lastSent = lastSentRef.current;
      const force = scheduleOptions?.force === true;

      if (!force && lastSent !== null && isEqual?.(value, lastSent)) {
        return;
      }

      pendingRef.current = value;

      if (force) {
        clearTimer();
        void flush();
        return;
      }

      // Ready/busy mode: if idle, flush immediately; if busy, keep only latest pending.
      if (interval === 0) {
        if (!inFlightRef.current) {
          clearTimer();
          void flush();
        }
        return;
      }

      if (timerRef.current !== null) return;

      timerRef.current = window.setTimeout(() => {
        timerRef.current = null;
        void flush();
      }, interval);
    },
    [flush, interval],
  );

  // Cleanup on unmount
  useEffect(() => cancel, [cancel]);

  return useMemo(
    () => ({ schedule, cancel }),
    [schedule, cancel],
  );
}
