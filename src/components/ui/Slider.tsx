import { ChangeEvent, CSSProperties } from 'react';

interface SliderProps {
  value: number;
  min?: number;
  max?: number;
  step?: number;
  onChange: (value: number) => void;
  onCommit?: (value: number) => void;
  disabled?: boolean;
  className?: string;
  style?: CSSProperties;
  id?: string;
}

export function Slider({
  value,
  min = 0,
  max = 100,
  step = 1,
  onChange,
  onCommit,
  disabled,
  className,
  style,
  id,
}: SliderProps) {
  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    const newValue = Number(e.target.value);
    onChange(newValue);
    if (onCommit) {
      onCommit(newValue);
    }
  };

  return (
    <input
      id={id}
      type="range"
      min={min}
      max={max}
      step={step}
      value={value}
      onChange={handleChange}
      disabled={disabled}
      className={className}
      style={{ 
        width: '100%', 
        accentColor: 'var(--accent-color)', 
        cursor: disabled ? 'not-allowed' : 'pointer', 
        ...style 
      }}
    />
  );
}

