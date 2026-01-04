import { HStack, Slider as ChakraSlider, For } from "@chakra-ui/react";
import { ReactNode } from "react";

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

  const handleValueChange = (details: ChakraSlider.ValueChangeDetails) => {
    onChange?.(details.value[0]);
  };

  const handleValueChangeEnd = (details: ChakraSlider.ValueChangeDetails) => {
    // 始终传递原始值，让调用方决定是否吸附及如何吸附（如带动画）
    onCommit?.(details.value[0]);
  };

  return (
    <ChakraSlider.Root
      min={min}
      max={max}
      step={step}
      value={[value]}
      onValueChange={handleValueChange}
      onValueChangeEnd={handleValueChangeEnd}
      disabled={disabled}
      colorPalette="blue"
    >
      {(label || valueText) && (
        <HStack justify="space-between">
          {label && <ChakraSlider.Label>{label}</ChakraSlider.Label>}
          {valueText && <ChakraSlider.ValueText>{valueText}</ChakraSlider.ValueText>}
        </HStack>
      )}
      <ChakraSlider.Control>
        <ChakraSlider.Track>
          <ChakraSlider.Range />
        </ChakraSlider.Track>
        <For each={[value]}>
          {(_, index) => (
            <ChakraSlider.Thumb key={index} index={index}>
              <ChakraSlider.HiddenInput />
            </ChakraSlider.Thumb>
          )}
        </For>
        {markerValues && markerValues.length > 0 && (
          <ChakraSlider.Marks marks={markerValues} />
        )}
      </ChakraSlider.Control>
    </ChakraSlider.Root>
  );
}

