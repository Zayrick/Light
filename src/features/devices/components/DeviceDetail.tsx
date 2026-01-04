import { useEffect, useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Sun, Sliders } from "lucide-react";
import { Device, EffectInfo, EffectParam, EffectParamValue, ScopeModeState } from "../../../types";
import type { SelectedScope } from "../../../types";
import { api } from "../../../services/api";
import { logger } from "../../../services/logger";
import { DeviceLedVisualizer } from "./DeviceLedVisualizer";
import { Card } from "../../../components/ui/Card";
import { Slider } from "../../../components/ui/Slider";
import { Tabs } from "../../../components/ui/Tabs";
import { ParamRenderer } from "./params/ParamRenderer";
import { checkDependency } from "../../../utils/effectUtils";
import { DynamicIcon } from "../../../components/DynamicIcon";

interface DeviceDetailProps {
  device: Device;
  scope: SelectedScope;
  effects: EffectInfo[];
  onRefresh: () => Promise<void>;
  onSelectScope?: (scope: SelectedScope) => void;
}

type ModeCategory = string;

interface DisplayMode extends EffectInfo {
  category: ModeCategory;
}

function buildParamState(effectId?: string, params?: Record<string, EffectParamValue>) {
  const initial: Record<string, EffectParamValue> = {};
  if (!effectId || !params) return initial;
  for (const [key, value] of Object.entries(params)) {
    if (typeof value === "number" || typeof value === "boolean" || typeof value === "string") {
      initial[`${effectId}:${key}`] = value;
    }
  }
  return initial;
}

function resolveScopeMode(device: Device, scope: SelectedScope): {
  title: string;
  subtitle?: string;
  mode: ScopeModeState;
  kind: "device" | "output" | "segment";
} {
  if (scope.outputId && scope.segmentId) {
    const out = device.outputs.find((o) => o.id === scope.outputId);
    const seg = out?.segments.find((s) => s.id === scope.segmentId);
    if (seg) {
      return {
        title: seg.name,
        subtitle: `${device.model} / ${out?.name ?? scope.outputId}`,
        mode: seg.mode,
        kind: "segment",
      };
    }
  }

  if (scope.outputId) {
    const out = device.outputs.find((o) => o.id === scope.outputId);
    if (out) {
      // Single-child compression:
      // If the device has only one output, show the device model as the title
      // to match the sidebar tree behavior.
      if (device.outputs.length === 1) {
        return {
          title: device.model,
          subtitle: out.name,
          mode: out.mode,
          kind: "output",
        };
      }

      return {
        title: out.name,
        subtitle: device.model,
        mode: out.mode,
        kind: "output",
      };
    }
  }

  return {
    title: device.model,
    subtitle: device.description,
    mode: device.mode,
    kind: "device",
  };
}

function formatScopeFrom(mode: ScopeModeState): string | null {
  const from = mode.effective_from;
  if (!from) return null;

  if (from.segment_id && from.output_id) {
    return `Segment ${from.output_id} / ${from.segment_id}`;
  }
  if (from.output_id) {
    return `Output ${from.output_id}`;
  }
  return "Device";
}

export function DeviceDetail({ device, scope, effects, onRefresh, onSelectScope }: DeviceDetailProps) {
  const resolvedScope = useMemo(() => resolveScopeMode(device, scope), [device, scope]);
  const scopeMode = resolvedScope.mode;

  const scopeKey = useMemo(() => {
    // Key used to animate transitions between scopes (device/output/segment) within the same device detail page.
    // Keep it stable and deterministic.
    return [scope.port, scope.outputId ?? "", scope.segmentId ?? ""].join("|");
  }, [scope.port, scope.outputId, scope.segmentId]);

  const effectiveModeId = scopeMode.effective_effect_id ?? null;
  const isInheriting =
    resolvedScope.kind !== "device" &&
    !scopeMode.selected_effect_id &&
    !!scopeMode.effective_effect_id;
  const fromLabel = formatScopeFrom(scopeMode);
  const showEffectiveFrom = !!fromLabel;
  // Reserve a fixed slot so the header height doesn't jump when Effective From appears.
  // The title/subtitle block will animate its vertical position to look vertically centered
  // when the slot is empty, and slide up when the slot is filled.
  const EFFECTIVE_FROM_SLOT_PX = 18;

  const [selectedCategory, setSelectedCategory] = useState<ModeCategory>(() => {
    const initialEffect = effects.find((e) => e.id === effectiveModeId);
    if (initialEffect?.group) return initialEffect.group;
    const firstGroup = effects.find((e) => e.group)?.group;
    return firstGroup ?? "Other";
  });

  const [selectedModeId, setSelectedModeId] = useState<string | null>(effectiveModeId);
  const [brightness, setBrightness] = useState(device.brightness ?? 100);
  const [paramValues, setParamValues] = useState<Record<string, EffectParamValue>>(() =>
    buildParamState(scopeMode.effective_effect_id, scopeMode.effective_params)
  );

  // Adapt raw backend effects into display modes with categories and icons
  const modes: DisplayMode[] = useMemo(() => {
    return effects.map((effect) => ({
      ...effect,
      category: effect.group ?? "Other",
    }));
  }, [effects]);

  const categories: ModeCategory[] = useMemo(() => {
    const set = new Set<ModeCategory>();
    modes.forEach((m) => set.add(m.category));
    return Array.from(set);
  }, [modes]);

  const modesByCategory = useMemo(() => {
    const map = new Map<ModeCategory, DisplayMode[]>();
    modes.forEach((mode) => {
      const list = map.get(mode.category) ?? [];
      list.push(mode);
      map.set(mode.category, list);
    });
    return map;
  }, [modes]);

  const selectedMode = modes.find((m) => m.id === selectedModeId);

  // Keep selected mode synced with effective selection (inheritance-aware)
  useEffect(() => {
    setSelectedModeId(effectiveModeId);
  }, [effectiveModeId, device.port, scope.outputId, scope.segmentId]);

  // Keep category synced
  useEffect(() => {
    const initialEffect = effects.find((e) => e.id === effectiveModeId);
    if (initialEffect?.group) {
      setSelectedCategory(initialEffect.group);
      return;
    }
    const firstGroup = effects.find((e) => e.group)?.group ?? "Other";
    setSelectedCategory(firstGroup);
  }, [effectiveModeId, effects]);

  useEffect(() => {
    if (!categories.includes(selectedCategory) && categories.length > 0) {
      setSelectedCategory(categories[0]);
    }
  }, [categories, selectedCategory]);

  // Sync brightness from backend device state
  useEffect(() => {
    setBrightness(device.brightness ?? 100);
  }, [device.brightness, device.port]);

  // Hydrate params from backend effective params
  useEffect(() => {
    setParamValues(buildParamState(scopeMode.effective_effect_id, scopeMode.effective_params));
  }, [
    device.port,
    scope.outputId,
    scope.segmentId,
    scopeMode.effective_effect_id,
    scopeMode.effective_params,
  ]);

  // Ensure defaults exist for current mode
  useEffect(() => {
    if (!selectedMode?.params?.length) return;
    setParamValues((prev) => {
      const next = { ...prev };
      let changed = false;
      selectedMode.params?.forEach((p) => {
        const key = `${selectedMode.id}:${p.key}`;
        if (!(key in next)) {
          next[key] = p.default;
          changed = true;
        }
      });
      return changed ? next : prev;
    });
  }, [selectedMode]);

  const getParamValue = (mode: DisplayMode, param: EffectParam) => {
    const key = `${mode.id}:${param.key}`;
    return paramValues[key] ?? param.default;
  };

  const handleBrightnessChange = async (value: number) => {
    setBrightness(value);
    try {
      await api.setBrightness(device.port, value);
      await onRefresh();
    } catch (err) {
      logger.error("device.brightness.set_failed", { port: device.port, brightness: value }, err);
    }
  };

  const handleModeClick = async (modeId: string) => {
    setSelectedModeId(modeId);
    try {
      await api.setScopeEffect({
        port: scope.port,
        outputId: scope.outputId,
        segmentId: scope.segmentId,
        effectId: modeId,
      });
      await onRefresh();
    } catch (err) {
      logger.error(
        "scope.effect.set_failed",
        { port: scope.port, outputId: scope.outputId, segmentId: scope.segmentId, effectId: modeId },
        err
      );
    }
  };

  const handleParamChange = (mode: DisplayMode, param: EffectParam, value: EffectParamValue) => {
    const storageKey = `${mode.id}:${param.key}`;
    setParamValues((prev) => ({ ...prev, [storageKey]: value }));
  };

  const handleParamCommit = async (mode: DisplayMode, param: EffectParam, value: EffectParamValue) => {
    try {
      await api.updateScopeEffectParams({
        port: scope.port,
        outputId: scope.outputId,
        segmentId: scope.segmentId,
        params: { [param.key]: value },
      });
      await onRefresh();
    } catch (err) {
      logger.error(
        "scope.effectParams.update_failed",
        { port: scope.port, outputId: scope.outputId, segmentId: scope.segmentId, effectId: mode.id, key: param.key },
        err
      );
    }
  };

  return (
    <AnimatePresence mode="wait" initial={false}>
      <motion.div
        key={scopeKey}
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -10 }}
        transition={{ duration: 0.22, ease: [0.16, 1, 0.3, 1] }}
        style={{
          height: "100%",
          display: "flex",
          flexDirection: "column",
          paddingTop: "56px",
          paddingBottom: "24px",
        }}
      >
      <header
        className="page-header"
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "16px",
        }}
      >
        <div
          style={{
            display: "flex",
            flexDirection: "column",
          }}
        >
          <motion.div
            layout="position"
            animate={showEffectiveFrom ? "expanded" : "collapsed"}
            variants={{
              collapsed: { y: EFFECTIVE_FROM_SLOT_PX / 2 },
              expanded: { y: 0 },
            }}
            transition={{ duration: 0.28, ease: [0.16, 1, 0.3, 1] }}
          >
            <h1 className="page-title" style={{ marginBottom: 0 }}>
              {resolvedScope.title}
            </h1>
            {resolvedScope.subtitle && <p className="page-subtitle">{resolvedScope.subtitle}</p>}
            <p className="page-subtitle" style={{ fontSize: "12px", opacity: 0.7 }}>
              SN: {device.id}
            </p>
          </motion.div>

          {/* Always reserve height for the Effective From line to prevent header reflow. */}
          <div
            style={{
              height: `${EFFECTIVE_FROM_SLOT_PX}px`,
              position: "relative",
              overflow: "hidden",
            }}
          >
            <AnimatePresence initial={false}>
              {showEffectiveFrom && (
                <motion.p
                  key="effective-from"
                  className="page-subtitle"
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: 6 }}
                  transition={{ duration: 0.22, ease: [0.16, 1, 0.3, 1] }}
                  style={{
                    margin: 0,
                    fontSize: "12px",
                    opacity: 0.7,
                    lineHeight: `${EFFECTIVE_FROM_SLOT_PX}px`,
                  }}
                >
                  Effective From: {fromLabel}
                  {isInheriting ? " (inheriting)" : ""}
                </motion.p>
              )}
            </AnimatePresence>
          </div>
        </div>
        <div
          style={{
            flex: 1,
            height: "80px",
            minWidth: "200px",
            maxWidth: "600px",
          }}
        >
          <DeviceLedVisualizer
            key={device.port}
            device={device}
            scope={scope}
            onSelectScope={onSelectScope}
          />
        </div>
      </header>

      <div style={{ display: "flex", gap: "24px", flex: 1, minHeight: 0 }}>
        {/* Left Column: Modes */}
        <div
          style={{
            flex: 1,
            display: "flex",
            flexDirection: "column",
            minHeight: 0,
            minWidth: 0,
          }}
        >
          <Tabs.Root
            value={selectedCategory}
            onValueChange={(details) => setSelectedCategory(details.value)}
            style={{
              flex: 1,
              minHeight: 0,
              display: "flex",
              flexDirection: "column",
              gap: "12px",
            }}
          >
            <Tabs.List>
              {categories.map((category) => (
                <Tabs.Trigger key={category} value={category}>
                  {category}
                </Tabs.Trigger>
              ))}
              <Tabs.Indicator />
            </Tabs.List>

            {categories.map((category) => {
              const categoryModes = modesByCategory.get(category) ?? [];
              return (
                <Tabs.Content key={category} value={category} style={{ minHeight: 0 }}>
                  <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
                    <div
                      className="no-scrollbar"
                      style={{ flex: 1, minHeight: 0, overflowY: "auto" }}
                    >
                      <div
                        style={{
                          display: "grid",
                          gridTemplateColumns: "repeat(auto-fill, minmax(160px, 1fr))",
                          gap: "8px",
                          padding: "2px",
                          paddingBottom: "20px",
                        }}
                      >
                        {categoryModes.map((mode) => {
                          const isSelected = selectedModeId === mode.id;
                          return (
                            <Card
                              key={mode.id}
                              hoverable
                              className={`${isSelected ? "active-mode-card" : ""}`}
                              style={{
                                border: isSelected
                                  ? "1px solid var(--accent-color)"
                                  : "1px solid transparent",
                                backgroundColor: isSelected ? "var(--bg-card-hover)" : undefined,
                                transition: "all 0.2s ease",
                                padding: "12px",
                              }}
                              onClick={() => handleModeClick(mode.id)}
                            >
                              <div
                                style={{
                                  display: "flex",
                                  flexDirection: "column",
                                  alignItems: "flex-start",
                                  gap: "10px",
                                }}
                              >
                                <div
                                  style={{
                                    width: "32px",
                                    height: "32px",
                                    borderRadius: "8px",
                                    display: "flex",
                                    alignItems: "center",
                                    justifyContent: "center",
                                    backgroundColor: isSelected
                                      ? "var(--accent-color)"
                                      : "rgba(128, 128, 128, 0.1)",
                                    color: isSelected
                                      ? "var(--accent-text)"
                                      : "var(--text-primary)",
                                    transition: "all 0.2s ease",
                                    boxShadow: isSelected ? "0 2px 8px rgba(0,0,0,0.2)" : "none",
                                  }}
                                >
                                  <DynamicIcon name={mode.icon || "Component"} size={18} />
                                </div>
                                <div style={{ display: "flex", flexDirection: "column", gap: "2px" }}>
                                  <div style={{ fontSize: "13px", fontWeight: 600 }}>{mode.name}</div>
                                  <div
                                    style={{
                                      fontSize: "11px",
                                      color: "var(--text-secondary)",
                                      lineHeight: "1.3",
                                    }}
                                  >
                                    {mode.description}
                                  </div>
                                </div>
                              </div>
                            </Card>
                          );
                        })}
                      </div>
                    </div>
                  </div>
                </Tabs.Content>
              );
            })}
          </Tabs.Root>
        </div>

        {/* Right Column: Configuration */}
        <div
          className="no-scrollbar fade-edges"
          style={{
            width: "280px",
            flex: "0 0 280px",
            minWidth: "260px",
            display: "flex",
            flexDirection: "column",
            gap: "12px",
            minHeight: 0,
            overflowY: "auto",
            paddingTop: "12px",
            paddingBottom: "12px",
          }}
        >
          {/* Global Device Settings */}
          <Card
            style={{
              padding: "16px",
              display: "flex",
              flexDirection: "column",
              gap: "12px",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <Sliders size={16} />
              <h3 style={{ margin: 0, fontSize: "14px", fontWeight: 600 }}>Device Settings</h3>
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  fontSize: "13px",
                  color: "var(--text-secondary)",
                }}
              >
                <span style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                  <Sun size={16} /> Brightness
                </span>
                <span>{brightness}%</span>
              </div>
              <Slider value={brightness} min={0} max={100} step={1} onChange={handleBrightnessChange} />
            </div>
          </Card>

          {/* Current Mode Settings */}
          {selectedMode && selectedMode.params && selectedMode.params.length > 0 && (
            <Card style={{ padding: "16px", display: "flex", flexDirection: "column", gap: "16px" }}>
              <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
                {selectedMode.params.map((param) => {
                  const value = getParamValue(selectedMode, param);
                  const { visible, disabled } = checkDependency(
                    selectedMode,
                    param.dependency,
                    paramValues
                  );

                  if (!visible) return null;

                  return (
                    <ParamRenderer
                      key={param.key}
                      param={param}
                      value={value}
                      disabled={disabled || isInheriting}
                      onChange={(val) => handleParamChange(selectedMode, param, val)}
                      onCommit={(val) => handleParamCommit(selectedMode, param, val)}
                    />
                  );
                })}
              </div>
            </Card>
          )}
        </div>
      </div>
      </motion.div>
    </AnimatePresence>
  );
}


