import { ComponentType, useMemo } from 'react';
import * as Icons from 'lucide-react';
import { LucideProps } from 'lucide-react';

interface DynamicIconProps extends LucideProps {
  name: string;
}

export function DynamicIcon({ name, ...props }: DynamicIconProps) {
  const Icon = useMemo(() => {
    // The backend should send the exact export name from lucide-react (PascalCase)
    // e.g. "Waves", "Monitor", "LayoutGrid"
    const iconName = name as keyof typeof Icons;
    const IconComponent = Icons[iconName];
    
    if (!IconComponent) {
        console.warn(`Icon "${name}" not found in lucide-react`);
        return Icons.Component; // Fallback icon
    }
    
    // We cast to specific type because keyof typeof Icons includes non-component exports if any (though usually it's just icons + createLucideIcon)
    return IconComponent as ComponentType<LucideProps>;
  }, [name]);

  return <Icon {...props} />;
}

