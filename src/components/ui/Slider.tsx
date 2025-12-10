import { Slider as ArkSlider } from "@ark-ui/react/slider";
import { ReactNode } from "react";
import "./Slider.css";

export interface SliderProps {
  /** 当前值 */
  value: number;
  /** 最小值 */
  min?: number;
  /** 最大值 */
  max?: number;
  /** 步进值 */
  step?: number;
  /** 可视化标记点（例如 Mipmap 采样点），值需在 min/max 范围内 */
  markers?: number[];
  /** 左侧标签 */
  label?: ReactNode;
  /** 右侧显示的值文本 */
  valueText?: ReactNode;
  /** 值变化时触发（拖动过程中） */
  onChange?: (value: number) => void;
  /** 值变化结束时触发（拖动结束） */
  onCommit?: (value: number) => void;
  /** 是否禁用 */
  disabled?: boolean;
}

export function Slider({
  value,
  min = 0,
  max = 100,
  step = 1,
  markers,
  label,
  valueText,
  onChange,
  onCommit,
  disabled = false,
}: SliderProps) {
  const markerValues = markers
    ?.filter((marker) => marker >= min && marker <= max)
    .sort((a, b) => a - b);

  const handleValueChange = (details: ArkSlider.ValueChangeDetails) => {
    onChange?.(details.value[0]);
  };

  const handleValueChangeEnd = (details: ArkSlider.ValueChangeDetails) => {
    // 始终传递原始值，让调用方决定是否吸附及如何吸附（如带动画）
    onCommit?.(details.value[0]);
  };

  return (
    <ArkSlider.Root
      min={min}
      max={max}
      step={step}
      value={[value]}
      onValueChange={handleValueChange}
      onValueChangeEnd={handleValueChangeEnd}
      disabled={disabled}
    >
      {(label || valueText) && (
        <div className="ark-slider-header">
          {label && <ArkSlider.Label>{label}</ArkSlider.Label>}
          {valueText && <span className="ark-slider-value">{valueText}</span>}
        </div>
      )}
      <ArkSlider.Control>
        <ArkSlider.Track>
          <ArkSlider.Range />
          {markerValues && markerValues.length > 0 && (
            <ArkSlider.MarkerGroup>
              {markerValues.map((marker) => (
                <ArkSlider.Marker key={marker} value={marker} />
              ))}
            </ArkSlider.MarkerGroup>
          )}
        </ArkSlider.Track>
        <ArkSlider.Thumb index={0} />
      </ArkSlider.Control>
    </ArkSlider.Root>
  );
}

