import { createSystem, defaultConfig, defineConfig, mergeConfigs } from "@chakra-ui/react";

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
          // 默认用 gray 调色板会让 focusRing 偏灰。
          // 这里切到 blue，并在 semanticTokens 覆盖其 focusRing 为应用的 accent。
          colorPalette: "blue",
        },
      },
      theme: {
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
            // 只做“轻度”调色：让 focus ring 跟随应用强调色。
            blue: {
              focusRing: { value: "var(--accent-color)" },
            },
          },
        },
      },
    }),
  ),
);
