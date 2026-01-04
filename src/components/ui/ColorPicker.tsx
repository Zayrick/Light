/**
 * Chakra UI v3 ColorPicker 封装
 *
 * 目标：完全复用 Chakra UI 的组件与样式（不写任何自定义样式）。
 * 这里仅做“值类型适配”：外部使用 string(hex)，Chakra 内部使用 Color 对象。
 */
import { ColorPicker as ChakraColorPicker, parseColor, Portal } from "@chakra-ui/react";
import { ReactNode, useMemo } from "react";

const FALLBACK_COLOR = "#ffffff";

export interface ColorPickerProps {
  value: string;
  label?: ReactNode;
  onChange?: (value: string) => void;
  onCommit?: (value: string) => void;
  disabled?: boolean;
}

function normalizeColorInput(input: string): string {
  const trimmed = input.trim();
  if (!trimmed) return FALLBACK_COLOR;
  if (!trimmed.startsWith("#") && !trimmed.includes("(")) return `#${trimmed}`;
  return trimmed;
}

export function ColorPicker({
  value,
  label,
  onChange,
  onCommit,
  disabled = false,
}: ColorPickerProps) {
  const parsedValue = useMemo(() => {
    try {
      return parseColor(normalizeColorInput(value));
    } catch {
      return parseColor(FALLBACK_COLOR);
    }
  }, [value]);

  return (
    <ChakraColorPicker.Root
      value={parsedValue}
      onValueChange={(details) => onChange?.(details.value.toString("hex"))}
      onValueChangeEnd={(details) => onCommit?.(details.value.toString("hex"))}
      disabled={disabled}
    >
      {label && <ChakraColorPicker.Label>{label}</ChakraColorPicker.Label>}

      <ChakraColorPicker.Control>
        <ChakraColorPicker.ChannelInput channel="hex" />
        <ChakraColorPicker.Trigger>
          <ChakraColorPicker.ValueSwatch />
        </ChakraColorPicker.Trigger>
      </ChakraColorPicker.Control>

      <Portal>
        <ChakraColorPicker.Positioner>
          <ChakraColorPicker.Content>
            <ChakraColorPicker.Area>
              <ChakraColorPicker.AreaBackground />
              <ChakraColorPicker.AreaThumb />
            </ChakraColorPicker.Area>

            <ChakraColorPicker.ChannelSlider channel="hue">
              <ChakraColorPicker.ChannelSliderTrack />
              <ChakraColorPicker.ChannelSliderThumb />
            </ChakraColorPicker.ChannelSlider>

            <ChakraColorPicker.ChannelSlider channel="alpha">
              <ChakraColorPicker.TransparencyGrid />
              <ChakraColorPicker.ChannelSliderTrack />
              <ChakraColorPicker.ChannelSliderThumb />
            </ChakraColorPicker.ChannelSlider>
          </ChakraColorPicker.Content>
        </ChakraColorPicker.Positioner>
      </Portal>

      <ChakraColorPicker.HiddenInput tabIndex={-1} />
    </ChakraColorPicker.Root>
  );
}
