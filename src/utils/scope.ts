import type { Device, SelectedScope } from "../types";

/**
 * Normalize a selected scope against the latest device snapshot.
 *
 * Single source of truth for the "single-output device" compression rule:
 * - If a device has exactly one output, selecting the device scope implies selecting that output.
 *
 * Also guards against stale output/segment IDs.
 */
export function normalizeSelectedScope(scope: SelectedScope, devices: Device[]): SelectedScope {
  const device = devices.find((d) => d.port === scope.port);
  if (!device) return scope;

  // If the device has a single output, always treat "device scope" as selecting the default output.
  if (!scope.outputId && device.outputs.length === 1) {
    return { port: scope.port, outputId: device.outputs[0].id };
  }

  if (scope.outputId) {
    const out = device.outputs.find((o) => o.id === scope.outputId);
    if (!out) {
      // Output no longer exists. Fall back to a stable scope.
      return device.outputs.length === 1
        ? { port: scope.port, outputId: device.outputs[0].id }
        : { port: scope.port };
    }

    if (scope.segmentId) {
      const segExists = out.segments.some((s) => s.id === scope.segmentId);
      if (!segExists) return { port: scope.port, outputId: scope.outputId };
    }
  }

  return scope;
}
