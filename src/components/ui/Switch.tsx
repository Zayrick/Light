import { Stack, Switch as ChakraSwitch, Text } from "@chakra-ui/react";
import type { ReactNode } from "react";

export interface SwitchProps {
  /** 当前是否开启 */
  checked: boolean;
  /** 状态变化 */
  onChange?: (checked: boolean) => void;
  /** 左侧主文案 */
  label?: ReactNode;
  /** 左侧描述（可选） */
  description?: ReactNode;
  /** 是否禁用 */
  disabled?: boolean;
}

export function Switch({
  checked,
  onChange,
  label,
  description,
  disabled = false,
}: SwitchProps) {
  return (
    <ChakraSwitch.Root
      checked={checked}
      disabled={disabled}
      onCheckedChange={(details) => onChange?.(details.checked)}
      display="flex"
      alignItems="center"
      justifyContent="space-between"
      gap="4"
      width="full"
    >
      {(label || description) && (
        <Stack gap="0.5" minW="0">
          {label && (
            <ChakraSwitch.Label fontSize="sm" fontWeight="medium">
              {label}
            </ChakraSwitch.Label>
          )}
          {description && (
            <Text fontSize="sm" color="fg.muted">
              {description}
            </Text>
          )}
        </Stack>
      )}

      <ChakraSwitch.HiddenInput />
      <ChakraSwitch.Control>
        <ChakraSwitch.Thumb />
      </ChakraSwitch.Control>
    </ChakraSwitch.Root>
  );
}
