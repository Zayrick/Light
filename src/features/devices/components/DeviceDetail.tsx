import { useState } from "react";
import { motion } from "framer-motion";
import { Device } from "../../../types";
import { Palette, Wind, Zap, Waves, Sparkles, Flame, Music, Monitor } from "lucide-react";

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

export function DeviceDetail({ device }: DeviceDetailProps) {
  const [selectedCategory, setSelectedCategory] = useState("Basic");

  const filteredModes = MOCK_MODES.filter(m => m.category === selectedCategory);

  return (
    <>
      <header className="page-header">
        <div>
          <h1 className="page-title" style={{ marginBottom: 0 }}>{device.model}</h1>
          <p className="page-subtitle">{device.description}</p>
          <p className="page-subtitle" style={{ fontSize: '12px', opacity: 0.7 }}>
            SN: {device.id}
          </p>
        </div>
      </header>

      <div style={{ marginTop: "40px" }}>
        {/* Categories */}
        <div className="mode-tabs">
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
          gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', 
          gap: '16px' 
        }}>
           {filteredModes.map(mode => (
              <div 
                key={mode.id} 
                className="device-card"
                style={{ cursor: 'pointer' }}
              >
                 <div className="device-icon" style={{ width: '36px', height: '36px', marginBottom: '4px' }}>
                    <mode.icon size={20} />
                 </div>
                 <div>
                   <div style={{ fontSize: '14px', fontWeight: 600 }}>{mode.name}</div>
                   <div style={{ fontSize: '12px', color: 'var(--text-secondary)', marginTop: '4px' }}>{mode.description}</div>
                 </div>
              </div>
           ))}
        </div>
      </div>
    </>
  );
}
