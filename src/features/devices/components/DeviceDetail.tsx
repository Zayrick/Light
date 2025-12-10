import { useEffect, useMemo, useState } from "react";
import { Device, EffectInfo, EffectParam } from "../../../types";
import { api } from "../../../services/api";
import { DeviceLedVisualizer } from "./DeviceLedVisualizer";
import { Sun, Sliders } from "lucide-react";
import { Card } from "../../../components/ui/Card";
import { Slider } from "../../../components/ui/Slider";
import { Tabs } from "../../../components/ui/Tabs";
import { ParamRenderer } from "./params/ParamRenderer";
import { checkDependency } from "../../../utils/effectUtils";
import { DynamicIcon } from "../../../components/DynamicIcon";

interface DeviceDetailProps {
  device: Device;
  effects: EffectInfo[];
  onSetEffect: (port: string, effectId: string) => Promise<void>;
}

type ModeCategory = string;

interface DisplayMode extends EffectInfo {
  category: ModeCategory;
}

export function DeviceDetail({
  device,
  effects,
  onSetEffect,
}: DeviceDetailProps) {
  const [selectedCategory, setSelectedCategory] = useState<ModeCategory>(() => {
    const initialEffect = effects.find(
      (e) => e.id === device.current_effect_id
    );
    if (initialEffect?.group) return initialEffect.group;

    // Default to the first available group from the backend data
    const firstGroup = effects.find((e) => e.group)?.group;
    return firstGroup ?? "Other";
  });
  const [selectedModeId, setSelectedModeId] = useState<string | null>(
    device.current_effect_id ?? null
  );

  // Mock settings states
  const [brightness, setBrightness] = useState(device.brightness ?? 100);
  const [paramValues, setParamValues] = useState<Record<string, number | boolean>>({});

  const commitBrightness = (value: number) => {
    api.setBrightness(device.port, value).catch(console.error);
  };

  // Adapt raw backend effects into display modes with categories and icons
  const modes: DisplayMode[] = useMemo(() => {
    return effects.map((effect) => {
      const category: ModeCategory = effect.group ?? "Other";

      return {
        ...effect,
        category,
      };
    });
  }, [effects]);

  // Compute available categories dynamically from backend groups
  // The order relies on the order of effects returned by the backend
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

  // Keep selected category valid if effects list changes
  useEffect(() => {
    if (!categories.includes(selectedCategory) && categories.length > 0) {
      setSelectedCategory(categories[0]);
    }
  }, [categories, selectedCategory]);

  // Sync selected mode with backend-known current effect for this device
  useEffect(() => {
    setSelectedModeId(device.current_effect_id ?? null);
  }, [device.current_effect_id, device.port]);

  // Ensure defaults exist for the selected mode so sliders have values
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

  const pushParamsToBackend = async (
    mode: DisplayMode,
    payload: Record<string, number | boolean>
  ) => {
    if (!mode.params || mode.params.length === 0) return;
    try {
      await api.updateEffectParams(device.port, payload);
    } catch (err) {
      console.error("Failed to update params:", err);
    }
  };

  const handleParamChange = (
    mode: DisplayMode,
    param: EffectParam,
    value: number | boolean
  ) => {
    const storageKey = `${mode.id}:${param.key}`;
    setParamValues((prev) => ({ ...prev, [storageKey]: value }));
  };

  const handleParamCommit = (
    mode: DisplayMode,
    param: EffectParam,
    value: number | boolean
  ) => {
    pushParamsToBackend(mode, { [param.key]: value });
  };

  const handleModeClick = async (modeId: string) => {
    setSelectedModeId(modeId);
    try {
      await onSetEffect(device.port, modeId);

      const mode = modes.find((m) => m.id === modeId);
      if (mode?.params?.length) {
        const payload: Record<string, number | boolean> = {};
        mode.params.forEach((p) => {
          payload[p.key] = getParamValue(mode, p);
        });
        await pushParamsToBackend(mode, payload);
      }
    } catch (err) {
      console.error("Failed to set effect:", err);
    }
  };

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <header
        className="page-header"
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "16px",
        }}
      >
        <div>
          <h1 className="page-title" style={{ marginBottom: 0 }}>
            {device.model}
          </h1>
          <p className="page-subtitle">{device.description}</p>
          <p
            className="page-subtitle"
            style={{ fontSize: "12px", opacity: 0.7 }}
          >
            SN: {device.id}
          </p>
        </div>
        <div
          style={{
            flex: 1,
            height: "80px",
            minWidth: "200px",
            maxWidth: "600px",
          }}
        >
          <DeviceLedVisualizer device={device} />
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
          {/* Categories (Mode Groups) */}
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
                <div
                  style={{
                    display: "flex",
                    flex: 1,
                    minHeight: 0,
                  }}
                >
                  <div
                    className="no-scrollbar"
                    style={{
                      flex: 1,
                      minHeight: 0,
                      overflowY: "auto",
                    }}
                  >
                    <div
                      style={{
                        display: "grid",
                        gridTemplateColumns: "repeat(auto-fill, minmax(160px, 1fr))",
                        gap: "8px",
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
                              backgroundColor: isSelected
                                ? "var(--bg-card-hover)"
                                : undefined,
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
                                  boxShadow: isSelected
                                    ? "0 2px 8px rgba(0,0,0,0.2)"
                                    : "none",
                                }}
                              >
                                <DynamicIcon name={mode.icon || "Component"} size={18} />
                              </div>
                              <div
                                style={{
                                  display: "flex",
                                  flexDirection: "column",
                                  gap: "2px",
                                }}
                              >
                                <div style={{ fontSize: "13px", fontWeight: 600 }}>
                                  {mode.name}
                                </div>
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

        {/* Right Column: Configuration (fixed/preferred width) */}
        <div
          className="no-scrollbar"
          style={{
            width: "280px",
            flex: "0 0 280px",
            minWidth: "260px",
            display: "flex",
            flexDirection: "column",
            gap: "12px",
            minHeight: 0,
            overflowY: "auto",
            paddingBottom: "20px",
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
              <h3 style={{ margin: 0, fontSize: "14px", fontWeight: 600 }}>
                Device Settings
              </h3>
            </div>

            {/* Brightness Control */}
            <div
              style={{ display: "flex", flexDirection: "column", gap: "10px" }}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  fontSize: "13px",
                  color: "var(--text-secondary)",
                }}
              >
                <span
                  style={{ display: "flex", alignItems: "center", gap: "6px" }}
                >
                  <Sun size={16} /> Brightness
                </span>
                <span>{brightness}%</span>
              </div>
              <Slider
                value={brightness}
                min={0}
                max={100}
                step={1}
                onChange={setBrightness}
                onCommit={commitBrightness}
              />
            </div>
          </Card>

          {/* Current Mode Settings */}
          {selectedMode &&
            selectedMode.params &&
            selectedMode.params.length > 0 && (
              <Card
                style={{
                  padding: "16px",
                  display: "flex",
                  flexDirection: "column",
                  gap: "16px",
                }}
              >
                <div
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: "16px",
                  }}
                >
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
                        disabled={disabled}
                        onChange={(val) =>
                          handleParamChange(selectedMode, param, val)
                        }
                        onCommit={(val) =>
                          handleParamCommit(selectedMode, param, val)
                        }
                      />
                    );
                  })}
                </div>
              </Card>
            )}
        </div>
      </div>
    </div>
  );
}
