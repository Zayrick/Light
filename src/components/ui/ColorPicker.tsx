/**
 * Chakra UI v3 ColorPicker 封装
 *
 * 目标：完全复用 Chakra UI 的组件与样式（尽量不写自定义样式）。
 * 这里仅做“值类型适配”：外部使用 string(hex)，Chakra 内部使用 Color 对象。
 */
import {
  ColorPicker as ChakraColorPicker,
  HStack,
  IconButton,
  Portal,
  parseColor,
} from "@chakra-ui/react";
import { Pipette } from "lucide-react";
import { ReactNode, useEffect, useMemo, useState } from "react";

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

function safeParseColor(input: string) {
  try {
    return parseColor(normalizeColorInput(input));
  } catch {
    return parseColor(FALLBACK_COLOR);
  }
}

export function ColorPicker({
  value,
  label,
  onChange,
  onCommit,
  disabled = false,
}: ColorPickerProps) {
  // 注意：hex 字符串无法表达 hue（例如 #000000 / 灰度色）。
  // 如果每次渲染都从 hex 重新 parse，会在拖到“最下方(亮度=0)”时丢失 hue，
  // Chakra 内部会回退到 hue=0，表现为“色相强制跳红”。
  // 这里用内部 Color 状态承载完整 HSVA 信息，并在外部回传同一 hex 时不覆盖内部状态。
  const externalColor = useMemo(() => safeParseColor(value), [value]);
  const externalHex = useMemo(() => externalColor.toString("hex"), [externalColor]);

  const [internalColor, setInternalColor] = useState(() => externalColor);
  const internalHex = useMemo(() => internalColor.toString("hex"), [internalColor]);

  useEffect(() => {
    // 仅当外部值与内部值确实不一致时才同步。
    // 这样父组件在拖动时回写同样的 hex（尤其是 #000000）不会把 hue 信息抹掉。
    if (externalHex !== internalHex) {
      setInternalColor(externalColor);
    }
  }, [externalColor, externalHex, internalHex]);

  const isEyeDropperSupported = useMemo(() => {
    // Chakra 的 EyeDropperTrigger 依赖浏览器 EyeDropper API。
    // 在 Tauri(Windows/WebView2) 通常可用，但仍做降级处理。
    return typeof window !== "undefined" && "EyeDropper" in window;
  }, []);

  return (
    <ChakraColorPicker.Root
      value={internalColor}
      onValueChange={(details) => {
        setInternalColor(details.value);
        onChange?.(details.value.toString("hex"));
      }}
      onValueChangeEnd={(details) => {
        setInternalColor(details.value);
        onCommit?.(details.value.toString("hex"));
      }}
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
          <ChakraColorPicker.Content bg="var(--bg-popover)" backdropFilter="none">
            <ChakraColorPicker.Area>
              <ChakraColorPicker.AreaBackground />
              <ChakraColorPicker.AreaThumb />
            </ChakraColorPicker.Area>

            <HStack gap="4" align="center">
              <ChakraColorPicker.EyeDropperTrigger asChild>
                <IconButton
                  aria-label="从屏幕取色"
                  size="xs"
                  variant="outline"
                  disabled={disabled || !isEyeDropperSupported}
                >
                  <Pipette size={14} />
                </IconButton>
              </ChakraColorPicker.EyeDropperTrigger>

              <ChakraColorPicker.ChannelSlider channel="hue" flex="1">
                <ChakraColorPicker.ChannelSliderTrack />
                <ChakraColorPicker.ChannelSliderThumb />
              </ChakraColorPicker.ChannelSlider>
            </HStack>
          </ChakraColorPicker.Content>
        </ChakraColorPicker.Positioner>
      </Portal>

      <ChakraColorPicker.HiddenInput tabIndex={-1} />
    </ChakraColorPicker.Root>
  );
}
