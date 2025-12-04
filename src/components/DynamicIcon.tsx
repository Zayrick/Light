import { ComponentType, useMemo } from 'react';
import {
  // Used by effects (from backend)
  AudioLines,
  Monitor,
  Power,
  Waves,
  LayoutGrid,
  // Fallback
  Component,
  // Other commonly used icons in the app
  Sun,
  Sliders,
  RefreshCw,
  Zap,
  ArrowRight,
  BookOpen,
  Puzzle,
  Music,
  Cast,
  Bell,
  ListFilter,
  Settings,
  Home,
  type LucideProps,
} from 'lucide-react';

// Explicit icon registry - only includes icons actually used in the app
// Add new icons here when needed by backend effects
const ICON_MAP: Record<string, ComponentType<LucideProps>> = {
  // Effect icons (from backend)
  AudioLines,
  Monitor,
  Power,
  Waves,
  LayoutGrid,
  // UI icons
  Component,
  Sun,
  Sliders,
  RefreshCw,
  Zap,
  ArrowRight,
  BookOpen,
  Puzzle,
  Music,
  Cast,
  Bell,
  ListFilter,
  Settings,
  Home,
};

interface DynamicIconProps extends LucideProps {
  name: string;
}

export function DynamicIcon({ name, ...props }: DynamicIconProps) {
  const Icon = useMemo(() => {
    const IconComponent = ICON_MAP[name];
    
    if (!IconComponent) {
      console.warn(`Icon "${name}" not found in ICON_MAP. Add it to DynamicIcon.tsx`);
      return Component; // Fallback icon
    }
    
    return IconComponent;
  }, [name]);

  return <Icon {...props} />;
}
