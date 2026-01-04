import { useEffect, useState } from "react";
import { EffectParam, EffectParamValue } from "../../../../types";
import { ColorRenderer } from "./ColorRenderer";
import { SelectRenderer } from "./SelectRenderer";
import { SliderRenderer } from "./SliderRenderer";
import { ToggleRenderer } from "./ToggleRenderer";

interface ParamRendererProps {
  param: EffectParam;
  value: EffectParamValue;
  disabled: boolean;
  /**
   * 高频实时变更（如 slider/颜色拖动）。
   * 注意：调用方应避免在这里 setState 触发大范围重渲染；推荐仅做节流后的后端同步。
   */
  onChange?: (value: EffectParamValue) => void;
  onCommit: (value: EffectParamValue) => void;
}

/**
 * Dispatcher component that decides which renderer to use based on param.type.
 *
 * 架构说明：
 * - ParamRenderer 统一管理 draft 状态，隔离拖动期的高频更新
 * - 各 Renderer 保持纯粹，只负责渲染与事件转发
 * - 默认只有 onCommit 才会冒泡到 DeviceDetail，避免拖动时触发整页重渲染
 * - 若传入 onChange，则用于“实时刷新”：在不阻塞 UI 的前提下，将高频变更节流同步到后端
 */
export function ParamRenderer({ param, value, disabled, onChange, onCommit }: ParamRendererProps) {
  // 本地 draft 状态：拖动期的高频更新只在这里消化，不冒泡到父组件
  const [draft, setDraft] = useState<EffectParamValue>(value);

  // 当外部 value 变化时同步（例如后端刷新、切换 effect）
  useEffect(() => {
    setDraft(value);
  }, [value]);

  const handleChange = (next: EffectParamValue) => {
    setDraft(next);
    onChange?.(next);
  };

  const handleCommit = (next: EffectParamValue) => {
    setDraft(next);
    onCommit(next);
  };

  switch (param.type) {
    case "slider":
      return (
        <SliderRenderer
          param={param}
          value={draft as number}
          disabled={disabled}
          onChange={handleChange as (v: number) => void}
          onCommit={handleCommit as (v: number) => void}
        />
      );
    case "select":
      return (
        <SelectRenderer
          param={param}
          value={draft as number}
          disabled={disabled}
          onCommit={handleCommit as (v: number) => void}
        />
      );
    case "toggle":
      return (
        <ToggleRenderer
          param={param}
          value={draft as boolean}
          disabled={disabled}
          onCommit={handleCommit as (v: boolean) => void}
        />
      );
    case "color":
      return (
        <ColorRenderer
          param={param}
          value={draft as string}
          disabled={disabled}
          onChange={handleChange as (v: string) => void}
          onCommit={handleCommit as (v: string) => void}
        />
      );
    default:
      console.warn(`No renderer found for param type: ${(param as EffectParam).type}`);
      return null;
  }
}
