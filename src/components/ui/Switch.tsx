import { Switch as ArkSwitch } from "@ark-ui/react/switch";
import type { ReactNode } from "react";
import "./Switch.css";

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
  const handleCheckedChange = (details: ArkSwitch.CheckedChangeDetails) => {
    onChange?.(details.checked);
  };

  return (
    <ArkSwitch.Root
      className="ark-switch-root"
      checked={checked}
      disabled={disabled}
      onCheckedChange={handleCheckedChange}
    >
      <span className="ark-switch-text">
        {label && <ArkSwitch.Label className="ark-switch-label">{label}</ArkSwitch.Label>}
        {description && <span className="ark-switch-description">{description}</span>}
      </span>
      <ArkSwitch.Control className="ark-switch-control">
        <ArkSwitch.Thumb className="ark-switch-thumb" />
      </ArkSwitch.Control>
      <ArkSwitch.HiddenInput />
    </ArkSwitch.Root>
  );
}
