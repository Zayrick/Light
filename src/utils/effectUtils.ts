import { EffectParam, ParamDependency } from "../types";

// Helper type for the display mode which includes params
interface EffectMode {
  id: string;
  params?: EffectParam[];
}

/**
 * Checks if a parameter's dependency is satisfied based on current values.
 * Returns visibility and disabled state.
 */
export function checkDependency(
  mode: EffectMode,
  dependency: ParamDependency | undefined,
  currentValues: Record<string, number>
): { visible: boolean; disabled: boolean } {
  if (!dependency) {
    return { visible: true, disabled: false };
  }

  // Simple behavior-only dependency (no key)
  if (!dependency.key) {
    if (dependency.behavior === "hide") {
      return { visible: false, disabled: false };
    } else if (dependency.behavior === "disable") {
      return { visible: true, disabled: true };
    }
    return { visible: true, disabled: false };
  }

  const controlling = mode.params?.find((p) => p.key === dependency.key);
  if (!controlling) {
    // Dependency key not found in params, assume satisfied or ignore
    return { visible: true, disabled: false };
  }

  // Construct key to lookup value
  const storageKey = `${mode.id}:${controlling.key}`;
  const controllingValue = currentValues[storageKey] ?? controlling.default;

  let met = true;

  if (
    dependency.equals !== undefined &&
    controllingValue !== dependency.equals
  ) {
    met = false;
  }
  if (
    dependency.notEquals !== undefined &&
    controllingValue === dependency.notEquals
  ) {
    met = false;
  }

  if (met) {
    return { visible: true, disabled: false };
  }

  if (dependency.behavior === "hide") {
    return { visible: false, disabled: false };
  }

  // default: disable when unmet
  return { visible: true, disabled: true };
}

