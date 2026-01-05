import { HStack } from "@chakra-ui/react";
import { ToggleLeft } from "lucide-react";
import { Switch } from "../../../../components/ui/Switch";
import { ToggleParam } from "../../../../types";

interface ToggleRendererProps {
  param: ToggleParam;
  value: boolean;
  disabled: boolean;
  onCommit: (value: boolean) => void;
}

/**
 * 纯渲染组件：Toggle 是离散切换，无拖动，直接 commit。
 */
export function ToggleRenderer({
  param,
  value,
  disabled,
  onCommit,
}: ToggleRendererProps) {
  return (
    <Switch
      checked={value}
      disabled={disabled}
      onChange={onCommit}
      label={
        <HStack gap="2">
          <ToggleLeft size={16} />
          <span>{param.label}</span>
        </HStack>
      }
    />
  );
}

