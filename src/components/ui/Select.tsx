import {
  Box,
  HStack,
  Portal,
  Select as ChakraSelect,
  Text,
  createListCollection,
} from "@chakra-ui/react";
import { ReactNode, useMemo } from "react";

export interface SelectOption<T extends string | number = string> {
  value: T;
  label: string;
}

export interface SelectProps<T extends string | number = string> {
  /** 当前选中的值 */
  value: T;
  /** 选项列表 */
  options: SelectOption<T>[];
  /** 左侧标签 */
  label?: ReactNode;
  /** 右侧显示的辅助文字 */
  valueText?: ReactNode;
  /** 值变化时触发 */
  onChange?: (value: T) => void;
  /** 是否禁用 */
  disabled?: boolean;
  /** placeholder */
  placeholder?: string;
}

export function Select<T extends string | number = string>({
  value,
  options,
  label,
  valueText,
  onChange,
  disabled = false,
  placeholder = "Select...",
}: SelectProps<T>) {
  const collection = useMemo(
    () =>
      createListCollection({
        items: options.map((opt) => ({
          value: String(opt.value),
          label: opt.label,
        })),
      }),
    [options],
  );

  const handleValueChange = (details: { value: string[] }) => {
    if (!details.value?.length) return;

    const rawValue = details.value[0];
    // 转换回原始类型
    const typedValue = (typeof value === "number" ? Number(rawValue) : rawValue) as T;
    onChange?.(typedValue);
  };

  return (
    <ChakraSelect.Root
      collection={collection}
      value={[String(value)]}
      onValueChange={handleValueChange}
      disabled={disabled}
      positioning={{ sameWidth: true }}
      size="sm"
      variant="outline"
    >
      <Box width="100%">
        {(label || valueText) && (
          <HStack justify="space-between" align="center" mb="2">
            {label && (
              <ChakraSelect.Label fontSize="sm" fontWeight="500" color="fg" lineHeight="1.2">
                {label}
              </ChakraSelect.Label>
            )}
            {valueText && (
              <Text fontSize="sm" color="fg.muted" opacity={0.7} lineHeight="1.2">
                {valueText}
              </Text>
            )}
          </HStack>
        )}

        <ChakraSelect.HiddenSelect />

        <ChakraSelect.Control width="full">
          <ChakraSelect.Trigger
            width="full"
            px="3"
            py="2"
            gap="2"
            justifyContent="space-between"
            border="1px solid"
            borderColor="border"
            borderRadius="var(--radius-m)"
            bg="bg.muted"
            color="fg"
            fontSize="14px"
            _hover={{ borderColor: "fg.muted" }}
            _focusVisible={{
              outline: "none",
              borderColor: "accent.solid",
              boxShadow:
                "0 0 0 2px color-mix(in srgb, var(--accent-color) 20%, transparent)",
            }}
          >
            <ChakraSelect.ValueText placeholder={placeholder} />
          </ChakraSelect.Trigger>
          <ChakraSelect.IndicatorGroup>
            <ChakraSelect.Indicator />
          </ChakraSelect.IndicatorGroup>
        </ChakraSelect.Control>
      </Box>

      <Portal>
        <ChakraSelect.Positioner>
          <ChakraSelect.Content
            p="1"
            bg="bg.panel"
            border="1px solid"
            borderColor="border"
            borderRadius="var(--radius-m)"
            boxShadow="0 8px 24px rgba(0, 0, 0, 0.3)"
            zIndex={100}
          >
            {collection.items.map((item) => (
              <ChakraSelect.Item
                key={item.value}
                item={item}
                px="3"
                py="2"
                borderRadius="var(--radius-s)"
                cursor="pointer"
                fontSize="14px"
                color="fg"
                _highlighted={{ bg: "var(--bg-card-hover)" }}
                css={{
                  "&[data-state='checked']": {
                    color: "var(--accent-color)",
                  },
                }}
              >
                <ChakraSelect.ItemText>{item.label}</ChakraSelect.ItemText>
                <ChakraSelect.ItemIndicator css={{ color: "var(--accent-color)" }} />
              </ChakraSelect.Item>
            ))}
          </ChakraSelect.Content>
        </ChakraSelect.Positioner>
      </Portal>
    </ChakraSelect.Root>
  );
}
