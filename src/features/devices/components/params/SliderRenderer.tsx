import { HStack, Slider, Text } from "@chakra-ui/react";
import { Gauge } from "lucide-react";
import { SliderParam } from "../../../../types";

interface SliderRendererProps {
  param: SliderParam;
  value: number;
  disabled: boolean;
  onChange: (value: number) => void;
  onCommit: (value: number) => void;
}

/**
 * 纯渲染组件：只负责渲染 Slider 并转发事件。
 * draft 状态由上层 ParamRenderer 统一管理。
 */
export function SliderRenderer({
  param,
  value,
  disabled,
  onChange,
  onCommit,
}: SliderRendererProps) {
  const formatParamValue = (p: SliderParam, v: number) => {
    if (p.step < 1) return v.toFixed(1);
    return Math.round(v).toString();
  };

  return (
    <Slider.Root
      min={param.min}
      max={param.max}
      step={param.step}
      value={[value]}
      onValueChange={(d) => onChange(d.value[0])}
      onValueChangeEnd={(d) => onCommit(d.value[0])}
      disabled={disabled}
    >
      <HStack justify="space-between">
        <Slider.Label>
          <HStack gap="1.5">
            <Gauge size={16} />
            <Text>{param.label}</Text>
          </HStack>
        </Slider.Label>
        <Slider.ValueText>{formatParamValue(param, value)}</Slider.ValueText>
      </HStack>
      <Slider.Control>
        <Slider.Track>
          <Slider.Range />
        </Slider.Track>
        <Slider.Thumbs />
      </Slider.Control>
    </Slider.Root>
  );
}

