import { createSystem, defaultConfig, defineConfig, defineSlotRecipe, mergeConfigs } from "@chakra-ui/react";
import { sliderAnatomy } from "@chakra-ui/react/anatomy";

/**
 * 自定义 Slider 组件样式
 * 覆盖 thumb 的背景色，使用与卡片一致的颜色（不透明）
 */
const sliderSlotRecipe = defineSlotRecipe({
  slots: sliderAnatomy.keys(),
  variants: {
    variant: {
      outline: {
        thumb: {
          // 使用 CSS 变量，自动适配明暗模式
          bg: "var(--slider-thumb-bg)",
        },
      },
    },
  },
});

/**
 * Chakra UI v3 默认会在 globalCss 里给 html 设置 `bg: "bg"`。
 * 在 Tauri 透明窗口 + Mica 场景下，这会让页面根节点变成不透明背景，视觉上像是 Mica 失效。
 *
 * 这里保持 Chakra 其它默认能力不变，仅覆盖 html 背景为 transparent。
 */
export const chakraSystem = createSystem(
  mergeConfigs(
    defaultConfig,
    defineConfig({
      globalCss: {
        html: {
          bg: "transparent",
          // 全局默认使用 accent 调色板，组件无需单独指定 colorPalette
          colorPalette: "accent",
        },
      },
      theme: {
        slotRecipes: {
          slider: sliderSlotRecipe,
        },
        semanticTokens: {
          colors: {
            // 让 Chakra 组件整体跟随应用的 CSS 变量（自动适配明暗主题）。
            fg: {
              DEFAULT: { value: "var(--text-primary)" },
              muted: { value: "var(--text-secondary)" },
            },
            bg: {
              // 仍保留窗口透明（Mica 由 Tauri 负责），组件自身的面板使用磨砂卡片/菜单底。
              DEFAULT: { value: "transparent" },
              muted: { value: "var(--bg-card)" },
              panel: { value: "var(--bg-context-menu)" },
            },
            border: {
              DEFAULT: { value: "var(--border-strong)" },
              muted: { value: "var(--border-subtle)" },
              subtle: { value: "var(--border-subtle)" },
            },
            // 自定义 accent 调色板，映射到应用的 CSS 主题变量
            // 这样所有使用 colorPalette="accent" 的组件都会自动使用正确的主题色
            accent: {
              solid: { value: "var(--accent-color)" },
              contrast: { value: "var(--accent-text)" },
              fg: { value: "var(--accent-color)" },
              muted: { value: "color-mix(in srgb, var(--accent-color) 20%, transparent)" },
              subtle: { value: "color-mix(in srgb, var(--accent-color) 15%, transparent)" },
              emphasized: { value: "var(--accent-hover)" },
              focusRing: { value: "var(--accent-color)" },
            },
          },
        },
      },
    }),
  ),
);
