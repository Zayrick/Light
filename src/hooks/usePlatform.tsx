import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { platform } from '@tauri-apps/plugin-os';

interface PlatformContextType {
  isMacOS: boolean;
  isWindows: boolean;
  isLinux: boolean;
  platform: string | null;
}

const PlatformContext = createContext<PlatformContextType>({
  isMacOS: false,
  isWindows: false,
  isLinux: false,
  platform: null,
});

export function PlatformProvider({ children }: { children: ReactNode }) {
  const [platformInfo, setPlatformInfo] = useState<PlatformContextType>(() => {
    // Initialize synchronously since platform() is synchronous
    try {
      const os = platform();
      return {
        isMacOS: os === 'macos',
        isWindows: os === 'windows',
        isLinux: os === 'linux',
        platform: os,
      };
    } catch {
      // Fallback for cases where the plugin might not be ready
      return {
        isMacOS: false,
        isWindows: false,
        isLinux: false,
        platform: null,
      };
    }
  });

  return (
    <PlatformContext.Provider value={platformInfo}>
      {children}
    </PlatformContext.Provider>
  );
}

export function usePlatform() {
  return useContext(PlatformContext);
}
