import { useState } from "react";
import { motion } from "framer-motion";
import { Device } from "../../../types";
import { 
  Palette, Wind, Zap, Waves, Sparkles, Flame, Music, Monitor,
  Sun, Gauge, RotateCw, Sliders
} from "lucide-react";
import { Card } from "../../../components/ui/Card";

interface DeviceDetailProps {
  device: Device;
}

// Mock Data
const MODE_CATEGORIES = ["Basic", "Dynamic", "Music", "Screen"];

const CATEGORY_TRANSITION = {
  duration: 0.25,
  ease: [0.16, 1, 0.3, 1] as const,
};

const MOCK_MODES = [
  { id: "static", name: "Static Color", category: "Basic", icon: Palette, description: "Solid color light" },
  { id: "breathing", name: "Breathing", category: "Basic", icon: Wind, description: "Fading in and out" },
  { id: "strobing", name: "Strobing", category: "Basic", icon: Zap, description: "Fast flashing" },
  
  { id: "rainbow", name: "Rainbow", category: "Dynamic", icon: Waves, description: "Flowing rainbow colors" },
  { id: "meteor", name: "Meteor", category: "Dynamic", icon: Sparkles, description: "Falling meteor trail" },
  { id: "fire", name: "Fire", category: "Dynamic", icon: Flame, description: "Flickering fire effect" },
  
  { id: "rhythm", name: "Rhythm", category: "Music", icon: Music, description: "Reacts to music beat" },
  
  { id: "sync", name: "Screen Sync", category: "Screen", icon: Monitor, description: "Matches screen colors" },
];

const MOCK_COLORS = ["#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#00FFFF", "#FF00FF", "#FFFFFF"];

export function DeviceDetail({ device }: DeviceDetailProps) {
  const [selectedCategory, setSelectedCategory] = useState("Basic");
  const [selectedModeId, setSelectedModeId] = useState("static");
  
  // Mock settings states
  const [brightness, setBrightness] = useState(80);
  const [speed, setSpeed] = useState(50);
  const [selectedColor, setSelectedColor] = useState("#FF0000");

  const filteredModes = MOCK_MODES.filter(m => m.category === selectedCategory);
  const selectedMode = MOCK_MODES.find(m => m.id === selectedModeId);

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <header className="page-header">
        <div>
          <h1 className="page-title" style={{ marginBottom: 0 }}>{device.model}</h1>
          <p className="page-subtitle">{device.description}</p>
          <p className="page-subtitle" style={{ fontSize: '12px', opacity: 0.7 }}>
            SN: {device.id}
          </p>
        </div>
      </header>

      <div style={{ display: 'flex', gap: '24px', flex: 1, minHeight: 0 }}>
        {/* Left Column: Modes */}
        <div className="no-scrollbar" style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0, overflowY: 'auto', }}>
          
          {/* Categories */}
          <div className="mode-tabs" style={{ marginTop: '0' }}>
            {MODE_CATEGORIES.map((category) => {
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
                      layoutId="mode-category-underline"
                      className="mode-tab-underline"
                      transition={CATEGORY_TRANSITION}
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
                    backgroundColor: isSelected ? 'var(--bg-card-hover)' : undefined
                  }}
                  onClick={() => setSelectedModeId(mode.id)}
                >
                   <div className="device-icon" style={{ 
                     width: '36px', height: '36px', marginBottom: '4px',
                     color: isSelected ? 'var(--accent-color)' : undefined 
                   }}>
                      <mode.icon size={20} />
                   </div>
                   <div>
                     <div style={{ fontSize: '14px', fontWeight: 600 }}>{mode.name}</div>
                     <div style={{ fontSize: '12px', color: 'var(--text-secondary)', marginTop: '4px' }}>{mode.description}</div>
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
                onChange={(e) => setBrightness(parseInt(e.target.value))}
                style={{ width: '100%', accentColor: 'var(--accent-color)' }}
              />
            </div>
          </Card>

          {/* Current Mode Settings */}
          {selectedMode && (
            <Card style={{ padding: '20px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '20px' }}>
                <selectedMode.icon size={18} />
                <h3 style={{ margin: 0, fontSize: '14px', fontWeight: 600 }}>{selectedMode.name} Config</h3>
              </div>

              {/* Configuration Controls based on mode type */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '24px' }}>
                
                {/* Speed Control (Dynamic Modes) */}
                {(selectedMode.category === 'Dynamic' || selectedMode.id === 'breathing' || selectedMode.id === 'strobing') && (
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
                {(selectedMode.category === 'Basic' || selectedMode.id === 'meteor') && (
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
