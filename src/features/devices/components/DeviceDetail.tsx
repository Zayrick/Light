import { useEffect, useMemo, useState, type ComponentType } from "react";
import { motion } from "framer-motion";
import { Device, EffectInfo } from "../../../types";
import { api } from "../../../services/api";
import { DeviceLedVisualizer } from "./DeviceLedVisualizer";
import { 
  Palette, Wind, Zap, Waves, Sparkles, Flame, Music, Monitor,
  Sun, Gauge, RotateCw, Sliders
} from "lucide-react";
import { Card } from "../../../components/ui/Card";

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

const MOCK_COLORS = ["#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#00FFFF", "#FF00FF", "#FFFFFF"];

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
  const [speed, setSpeed] = useState(50);
  const [selectedColor, setSelectedColor] = useState("#FF0000");
  const [hasMounted, setHasMounted] = useState(false);

  useEffect(() => {
    // Avoid underline "floating" on initial page enter; only animate on user interactions
    setHasMounted(true);
  }, []);

  const handleBrightnessChange = (value: number) => {
    setBrightness(value);
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

  const filteredModes = modes.filter((m) => m.category === selectedCategory);

  const selectedMode = modes.find((m) => m.id === selectedModeId);

  const handleModeClick = async (modeId: string) => {
    setSelectedModeId(modeId);
    try {
      await onSetEffect(device.port, modeId);
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
        <div className="no-scrollbar" style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0, overflowY: 'auto', }}>
          
          {/* Categories */}
          <div className="mode-tabs" style={{ marginTop: '0' }}>
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

          {/* Modes Grid */}
          <div style={{ 
            display: 'grid', 
            gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))', 
            gap: '12px',
            paddingBottom: '20px'
          }}>
             {filteredModes.map(mode => {
               const isSelected = selectedModeId === mode.id;
               return (
                <Card 
                  key={mode.id} 
                  hoverable
                  className={`${isSelected ? 'active-mode-card' : ''}`}
                  style={{ 
                    border: isSelected ? '1px solid var(--accent-color)' : '1px solid transparent',
                    backgroundColor: isSelected ? 'var(--bg-card-hover)' : undefined,
                    transition: 'all 0.2s ease'
                  }}
                  onClick={() => handleModeClick(mode.id)}
                >
                   <div style={{ 
                     display: 'flex',
                     flexDirection: 'column',
                     alignItems: 'flex-start',
                     gap: '12px'
                   }}>
                     <div style={{ 
                       width: '40px', 
                       height: '40px', 
                       borderRadius: '10px',
                       display: 'flex',
                       alignItems: 'center',
                       justifyContent: 'center',
                       backgroundColor: isSelected ? 'var(--accent-color)' : 'rgba(128, 128, 128, 0.1)',
                       color: isSelected ? 'var(--accent-text)' : 'var(--text-primary)',
                       transition: 'all 0.2s ease',
                       boxShadow: isSelected ? '0 2px 8px rgba(0,0,0,0.2)' : 'none'
                     }}>
                        <mode.icon size={20} />
                     </div>
                     <div>
                       <div style={{ fontSize: '14px', fontWeight: 600, marginBottom: '4px' }}>{mode.name}</div>
                       <div style={{ fontSize: '12px', color: 'var(--text-secondary)', lineHeight: '1.4' }}>{mode.description}</div>
                     </div>
                   </div>
                </Card>
               );
             })}
          </div>
        </div>

        {/* Right Column: Configuration */}
        <div className="no-scrollbar" style={{ width: '320px', display: 'flex', flexDirection: 'column', gap: '16px', minHeight: 0, overflowY: 'auto', paddingBottom: '20px' }}>
          
          {/* Global Device Settings */}
          <Card style={{ padding: '20px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '16px' }}>
              <Sliders size={18} />
              <h3 style={{ margin: 0, fontSize: '14px', fontWeight: 600 }}>Device Settings</h3>
            </div>
            
            {/* Brightness Control */}
            <div style={{ marginBottom: '12px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px', fontSize: '12px', color: 'var(--text-secondary)' }}>
                <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}><Sun size={12} /> Brightness</span>
                <span>{brightness}%</span>
              </div>
              <input 
                type="range" 
                min="0" 
                max="100" 
                value={brightness} 
                onChange={(e) => handleBrightnessChange(parseInt(e.target.value))}
                style={{ width: '100%', accentColor: 'var(--accent-color)' }}
              />
            </div>
          </Card>

          {/* Current Mode Settings */}
          {selectedMode && (
            <Card style={{ padding: '20px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '12px', marginBottom: '20px' }}>
                <div style={{
                  width: '32px', height: '32px',
                  borderRadius: '8px',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  backgroundColor: 'var(--accent-color)',
                  color: 'var(--accent-text)',
                  boxShadow: '0 2px 8px rgba(0,0,0,0.15)'
                }}>
                   <selectedMode.icon size={18} />
                </div>
                <h3 style={{ margin: 0, fontSize: '15px', fontWeight: 600 }}>{selectedMode.name} Config</h3>
              </div>

              {/* Configuration Controls based on mode type */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
                
                {/* Speed Control (Dynamic Modes) */}
                {selectedMode.category === 'Dynamic' && (
                  <div>
                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px', fontSize: '12px', color: 'var(--text-secondary)' }}>
                       <span style={{ display: 'flex', alignItems: 'center', gap: '4px' }}><Gauge size={12} /> Speed</span>
                       <span>{speed}%</span>
                    </div>
                    <input 
                      type="range" 
                      min="1" 
                      max="100" 
                      value={speed} 
                      onChange={(e) => setSpeed(parseInt(e.target.value))}
                      style={{ width: '100%', accentColor: 'var(--accent-color)' }}
                    />
                  </div>
                )}

                {/* Direction Control (Dynamic Modes) */}
                {selectedMode.category === 'Dynamic' && (
                  <div>
                    <div style={{ marginBottom: '8px', fontSize: '12px', color: 'var(--text-secondary)', display: 'flex', alignItems: 'center', gap: '4px' }}>
                      <RotateCw size={12} /> Direction
                    </div>
                    <div style={{ display: 'flex', gap: '8px' }}>
                      <button className="btn btn-secondary" style={{ flex: 1, fontSize: '12px' }}>Clockwise</button>
                      <button className="btn btn-secondary" style={{ flex: 1, fontSize: '12px', opacity: 0.5 }}>Counter</button>
                    </div>
                  </div>
                )}

                {/* Color Control (Static/Basic Modes) */}
                {selectedMode.category === 'Basic' && (
                  <div>
                    <div style={{ marginBottom: '8px', fontSize: '12px', color: 'var(--text-secondary)', display: 'flex', alignItems: 'center', gap: '4px' }}>
                      <Palette size={12} /> Color
                    </div>
                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px' }}>
                      {MOCK_COLORS.map(color => (
                        <div 
                          key={color}
                          onClick={() => setSelectedColor(color)}
                          style={{
                            width: '24px',
                            height: '24px',
                            borderRadius: '50%',
                            backgroundColor: color,
                            cursor: 'pointer',
                            border: selectedColor === color ? '2px solid var(--text-primary)' : '1px solid var(--border-strong)',
                            boxShadow: selectedColor === color ? '0 0 0 2px var(--bg-card)' : 'none',
                            transition: 'all 0.2s'
                          }}
                        />
                      ))}
                      <div style={{ 
                        width: '24px', 
                        height: '24px', 
                        borderRadius: '50%', 
                        background: 'conic-gradient(red, yellow, lime, aqua, blue, magenta, red)',
                        cursor: 'pointer',
                        border: '1px solid var(--border-strong)'
                      }} title="Custom" />
                    </div>
                  </div>
                )}
                
                {/* Placeholder for other modes */}
                {selectedMode.category === 'Screen' && (
                   <div style={{ fontSize: '12px', color: 'var(--text-secondary)', fontStyle: 'italic' }}>
                     Screen synchronization settings will appear here.
                   </div>
                )}

              </div>
            </Card>
          )}

        </div>
      </div>
    </div>
  );
}
