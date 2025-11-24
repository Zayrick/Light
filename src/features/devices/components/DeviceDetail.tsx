import {
  useEffect,
  useMemo,
  useState,
  useRef,
  type ComponentType,
  type WheelEvent,
} from "react";
import { motion } from "framer-motion";
import FormControl from "@mui/material/FormControl";
import Select, { SelectChangeEvent } from "@mui/material/Select";
import MenuItem from "@mui/material/MenuItem";
import {
  Device,
  EffectInfo,
  EffectParam,
  ParamDependency,
  SliderParam,
} from "../../../types";
import { api } from "../../../services/api";
import { DeviceLedVisualizer } from "./DeviceLedVisualizer";
import {
  Palette,
  Zap,
  Waves,
  Sparkles,
  Monitor,
  Sun,
  Gauge,
  Sliders,
  ListFilter,
} from "lucide-react";
import { Card } from "../../../components/ui/Card";
import { Slider } from "../../../components/ui/Slider";

interface DeviceDetailProps {
  device: Device;
  effects: EffectInfo[];
  onSetEffect: (port: string, effectId: string) => Promise<void>;
}

type ModeCategory = string;

const CATEGORY_TRANSITION = {
  duration: 0.25,
  ease: [0.16, 1, 0.3, 1] as const,
};

// Icon mapping helpers so visual style stays nice while data is real
const EFFECT_ICON_MAP: Record<string, ComponentType<{ size?: number }>> = {
  rainbow: Waves,
  matrix_test: Monitor,
  turn_off: Sun,
};

const GROUP_ICON_MAP: Record<string, ComponentType<{ size?: number }>> = {
  Basic: Palette,
  Dynamic: Waves,
  Test: Sparkles,
};

const DEFAULT_ICON = Zap;

interface DisplayMode extends EffectInfo {
  category: ModeCategory;
  icon: ComponentType<{ size?: number }>;
}

export function DeviceDetail({ device, effects, onSetEffect }: DeviceDetailProps) {
  const [selectedCategory, setSelectedCategory] = useState<ModeCategory>(() => {
    const initialEffect = effects.find((e) => e.id === device.current_effect_id);
    if (initialEffect?.group) return initialEffect.group;
    const groups = Array.from(
      new Set(effects.map((e) => e.group ?? "Other"))
    );
    const preferredOrder = ["Basic", "Dynamic", "Music", "Screen", "Test", "Other"];
    for (const g of preferredOrder) {
      if (groups.includes(g)) return g;
    }
    return groups[0] ?? "Other";
  });
  const [selectedModeId, setSelectedModeId] = useState<string | null>(device.current_effect_id ?? null);
  
  // Mock settings states
  const [brightness, setBrightness] = useState(device.brightness ?? 100);
  const [paramValues, setParamValues] = useState<Record<string, number>>({});
  const [hasMounted, setHasMounted] = useState(false);

  const categoryScrollRef = useRef<HTMLDivElement | null>(null);

  const handleCategoryWheel = (event: WheelEvent<HTMLDivElement>) => {
    const container = categoryScrollRef.current;
    if (!container) return;

    // Translate vertical wheel scrolling into horizontal scrolling
    if (Math.abs(event.deltaY) > Math.abs(event.deltaX)) {
      container.scrollLeft += event.deltaY;
      event.preventDefault();
    }
  };

  useEffect(() => {
    // Avoid underline "floating" on initial page enter; only animate on user interactions
    setHasMounted(true);
  }, []);

  const commitBrightness = (value: number) => {
    api.setBrightness(device.port, value).catch(console.error);
  };

  // Adapt raw backend effects into display modes with categories and icons
  const modes: DisplayMode[] = useMemo(() => {
    return effects.map((effect) => {
      const category: ModeCategory = effect.group ?? "Other";
      const icon =
        EFFECT_ICON_MAP[effect.id] ??
        (effect.group ? GROUP_ICON_MAP[effect.group] : undefined) ??
        DEFAULT_ICON;

      return {
        ...effect,
        category,
        icon,
      };
    });
  }, [effects]);

  // Compute available categories dynamically from backend groups
  const categories: ModeCategory[] = useMemo(() => {
    const set = new Set<ModeCategory>();
    modes.forEach((m) => set.add(m.category));
    const available = Array.from(set);
    const preferredOrder = ["Basic", "Dynamic", "Music", "Screen", "Test", "Other"];
    const ordered = preferredOrder.filter((c) => available.includes(c));
    const remaining = available.filter((c) => !preferredOrder.includes(c));
    return [...ordered, ...remaining];
  }, [modes]);

  const filteredModes = modes.filter((m) => m.category === selectedCategory);
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

  const isDependencySatisfied = (
    mode: DisplayMode,
    dependency?: ParamDependency
  ): { visible: boolean; disabled: boolean } => {
    if (!dependency) {
      return { visible: true, disabled: false };
    }

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
      return { visible: true, disabled: false };
    }

    const controllingValue = getParamValue(mode, controlling);
    let met = true;

    if (dependency.equals !== undefined && controllingValue !== dependency.equals) {
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
  };

  const formatParamValue = (param: SliderParam, value: number) => {
    if (param.step < 1) return value.toFixed(1);
    return Math.round(value).toString();
  };

  const pushParamsToBackend = async (mode: DisplayMode, payload: Record<string, number>) => {
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
    value: number
  ) => {
    const storageKey = `${mode.id}:${param.key}`;
    setParamValues((prev) => ({ ...prev, [storageKey]: value }));
  };

  const handleParamCommit = (
    mode: DisplayMode,
    param: EffectParam,
    value: number
  ) => {
    pushParamsToBackend(mode, { [param.key]: value });
  };

  const handleModeClick = async (modeId: string) => {
    setSelectedModeId(modeId);
    try {
      await onSetEffect(device.port, modeId);

      const mode = modes.find((m) => m.id === modeId);
      if (mode?.params?.length) {
        const payload: Record<string, number> = {};
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
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <header className="page-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '16px' }}>
        <div>
          <h1 className="page-title" style={{ marginBottom: 0 }}>{device.model}</h1>
          <p className="page-subtitle">{device.description}</p>
          <p className="page-subtitle" style={{ fontSize: '12px', opacity: 0.7 }}>
            SN: {device.id}
          </p>
        </div>
        <div style={{ flex: 1, height: '80px', minWidth: '200px', maxWidth: '600px' }}>
          <DeviceLedVisualizer device={device} />
        </div>
      </header>

      <div style={{ display: 'flex', gap: '24px', flex: 1, minHeight: 0 }}>
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
          <div
            ref={categoryScrollRef}
            className="mode-tabs no-scrollbar"
            style={{ marginTop: "0" }}
            onWheel={handleCategoryWheel}
          >
            {categories.map((category) => {
              const isActive = selectedCategory === category;
              return (
                <motion.button
                  key={category}
                  type="button"
                  onClick={() => setSelectedCategory(category)}
                  className={`mode-tab-button ${isActive ? "mode-tab-button-active" : ""}`}
                  animate={{
                    opacity: isActive ? 1 : 0.6,
                    scale: isActive ? 1 : 0.98,
                  }}
                  whileHover={{ opacity: 1 }}
                  transition={CATEGORY_TRANSITION}
                >
                  {isActive && (
                    <motion.div
                      layoutId={`mode-category-underline-${device.id}`}
                      className="mode-tab-underline"
                      transition={hasMounted ? CATEGORY_TRANSITION : { duration: 0 }}
                    />
                  )}
                  <span className="mode-tab-label">{category}</span>
                </motion.button>
              );
            })}
          </div>

          {/* Modes Grid (independent vertical scroll area) */}
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
              {filteredModes.map((mode) => {
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
                        <mode.icon size={18} />
                      </div>
                      <div
                        style={{
                          display: "flex",
                          flexDirection: "column",
                          gap: "2px",
                        }}
                      >
                        <div
                          style={{ fontSize: "13px", fontWeight: 600 }}
                        >
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
          <Card style={{ padding: '16px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <Sliders size={16} />
              <h3 style={{ margin: 0, fontSize: '13px', fontWeight: 600 }}>Device Settings</h3>
            </div>
            
            {/* Brightness Control */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "11px", color: "var(--text-secondary)" }}>
                <span style={{ display: "flex", alignItems: "center", gap: "4px" }}><Sun size={11} /> Brightness</span>
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
          {selectedMode && selectedMode.params && selectedMode.params.length > 0 && (
            <Card style={{ padding: '16px', display: 'flex', flexDirection: 'column', gap: '16px' }}>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                {selectedMode.params?.map((param) => {
                  if (param.type === 'slider') {
                    const value = getParamValue(selectedMode, param);
                    const { visible, disabled } = isDependencySatisfied(
                      selectedMode,
                      param.dependency
                    );
                    if (!visible) return null;
                    return (
                      <div key={param.key} style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: 'var(--text-secondary)' }}>
                          <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}><Gauge size={11} /> {param.label}</span>
                          <span>{formatParamValue(param, value)}</span>
                        </div>
                        <Slider
                          value={value}
                          min={param.min}
                          max={param.max}
                          step={param.step}
                          disabled={disabled}
                          onChange={(newValue) =>
                            handleParamChange(selectedMode, param, newValue)
                          }
                          onCommit={(newValue) =>
                            handleParamCommit(selectedMode, param, newValue)
                          }
                        />
                      </div>
                    );
                  } else if (param.type === 'select') {
                    const value = getParamValue(selectedMode, param);
                    const hasOptions = param.options.length > 0;
                    const selectLabelId = `${selectedMode.id}-${param.key}-label`;
                    const { visible, disabled } = isDependencySatisfied(
                      selectedMode,
                      param.dependency
                    );
                    if (!visible) return null;
                    return (
                      <div key={param.key} style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                        <div
                          style={{
                            display: 'flex',
                            justifyContent: 'space-between',
                            fontSize: '11px',
                            color: 'var(--text-secondary)',
                            alignItems: 'center',
                          }}
                        >
                          <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}>
                            <ListFilter size={11} /> {param.label}
                          </span>
                          {hasOptions && (
                            <span style={{ opacity: 0.7 }}>
                              {param.options.length} option{param.options.length > 1 ? 's' : ''}
                            </span>
                          )}
                        </div>
                        {hasOptions ? (
                          <FormControl fullWidth size="small" variant="outlined">
                            <Select
                              labelId={selectLabelId}
                              value={String(value)}
                              disabled={disabled}
                              onChange={(event: SelectChangeEvent<string>) => {
                                const val = Number(event.target.value);
                                handleParamChange(selectedMode, param, val);
                                handleParamCommit(selectedMode, param, val);
                              }}
                              MenuProps={{
                                PaperProps: {
                                  sx: {
                                    maxHeight: 280,
                                    backgroundColor: "var(--bg-card)",
                                    backdropFilter: "blur(20px)",
                                    color: "var(--text-primary)",
                                    borderRadius: "var(--radius-m)",
                                    border: "1px solid var(--border-subtle)",
                                    "& .MuiMenuItem-root": {
                                      "&.Mui-selected": {
                                        backgroundColor: "var(--accent-color)",
                                        color: "var(--accent-text)",
                                        "&:hover": {
                                          backgroundColor: "var(--accent-hover)",
                                        },
                                      },
                                      "&:hover": {
                                        backgroundColor: "var(--bg-card-hover)",
                                      },
                                      fontSize: '13px', // Compact menu items
                                      minHeight: '32px',
                                    },
                                  },
                                },
                              }}
                              sx={{
                                color: "var(--text-primary)",
                                borderRadius: "var(--radius-m)",
                                fontSize: "13px", // Smaller text
                                height: "32px",   // Reduced height
                                ".MuiOutlinedInput-notchedOutline": {
                                  borderColor: "var(--border-subtle)",
                                },
                                "&:hover .MuiOutlinedInput-notchedOutline": {
                                  borderColor: "var(--text-secondary)",
                                },
                                "&.Mui-focused .MuiOutlinedInput-notchedOutline": {
                                  borderColor: "var(--accent-color)",
                                },
                                ".MuiSvgIcon-root": {
                                  color: "var(--text-secondary)",
                                },
                                ".MuiSelect-select": {
                                  display: "flex",
                                  alignItems: "center",
                                  paddingTop: "4px",
                                  paddingBottom: "4px",
                                },
                              }}
                            >
                              {param.options.map((option) => (
                                <MenuItem key={option.value} value={String(option.value)}>
                                  {option.label}
                                </MenuItem>
                              ))}
                            </Select>
                          </FormControl>
                        ) : (
                          <div style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>
                            No options available.
                          </div>
                        )}
                      </div>
                    );
                  }
                  return null;
                })}
              </div>
            </Card>
          )}

        </div>
      </div>
    </div>
  );
}
