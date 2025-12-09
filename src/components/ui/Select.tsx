import { Portal } from "@ark-ui/react/portal";
import { Select as ArkSelect, createListCollection } from "@ark-ui/react/select";
import { ChevronDownIcon } from "lucide-react";
import { ReactNode, useMemo } from "react";
import "./Select.css";

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
    [options]
  );

  const handleValueChange = (details: ArkSelect.ValueChangeDetails) => {
    if (details.value.length > 0) {
      const rawValue = details.value[0];
      // 转换回原始类型
      const typedValue = (typeof value === "number" ? Number(rawValue) : rawValue) as T;
      onChange?.(typedValue);
    }
  };

  return (
    <ArkSelect.Root
      collection={collection}
      value={[String(value)]}
      onValueChange={handleValueChange}
      disabled={disabled}
      positioning={{ sameWidth: true }}
    >
      {(label || valueText) && (
        <div className="ark-select-header">
          {label && <ArkSelect.Label>{label}</ArkSelect.Label>}
          {valueText && <span className="ark-select-value-text">{valueText}</span>}
        </div>
      )}
      <ArkSelect.Control>
        <ArkSelect.Trigger>
          <ArkSelect.ValueText placeholder={placeholder} />
          <ChevronDownIcon size={16} />
        </ArkSelect.Trigger>
      </ArkSelect.Control>
      <Portal>
        <ArkSelect.Positioner>
          <ArkSelect.Content>
            {collection.items.map((item) => (
              <ArkSelect.Item key={item.value} item={item}>
                <ArkSelect.ItemText>{item.label}</ArkSelect.ItemText>
                <ArkSelect.ItemIndicator>✓</ArkSelect.ItemIndicator>
              </ArkSelect.Item>
            ))}
          </ArkSelect.Content>
        </ArkSelect.Positioner>
      </Portal>
    </ArkSelect.Root>
  );
}
