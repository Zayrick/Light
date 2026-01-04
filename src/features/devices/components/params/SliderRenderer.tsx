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

export function SliderRenderer({
  param,
  value,
  disabled,
  onChange,
  onCommit,
}: SliderRendererProps) {
  const handleChange = (details: Slider.ValueChangeDetails) => {
    const nextValue = details.value[0];
    onChange(nextValue);
    onCommit(nextValue);
  };

  const formatParamValue = (param: SliderParam, value: number) => {
    if (param.step < 1) return value.toFixed(1);
    return Math.round(value).toString();
  };

  return (
    <Slider.Root
      min={param.min}
      max={param.max}
      step={param.step}
      value={[value]}
      onValueChange={handleChange}
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

