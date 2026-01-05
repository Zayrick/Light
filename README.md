# Light

Light 是一个基于 **Tauri** (Rust) 和 **React** 构建的现代化、跨平台 RGB 灯光控制应用程序。

它旨在提供一个高性能、可扩展且架构清晰的解决方案，用于管理各种 RGB 设备（如串口设备、HID 设备、UDP 矩阵等）并应用复杂的灯光效果（如屏幕镜像、音频可视化等）。

## ✨ 核心特性

*   **跨平台支持**：支持 Windows, macOS 和 Linux。
*   **高性能后端**：基于 Rust 构建，利用 `inventory` crate 实现去中心化的插件式架构。
*   **后端驱动 UI**：前端 UI 根据后端定义的能力和参数动态生成，无需硬编码。
*   **丰富的灯效库**：
    *   🌈 **Rainbow**: 经典的彩虹波浪效果。
    *   🖥️ **Screen Mirror**: 实时屏幕色彩同步（支持 Windows/macOS/Linux 原生 API）。
    *   🎵 **Audio Star**: 基于音频频谱分析的律动效果。
    *   💡 **Matrix Test**: LED 矩阵测试工具。
*   **多设备支持**：
    *   支持串口设备 (如 Skydimo)。
    *   支持 HID 设备 (如 DRGB 控制器)。
    *   支持网络设备 (如 UDP LED 矩阵)。
*   **可视化预览**：前端使用 PixiJS 提供实时的 LED 布局和灯效预览。

## 🏗️ 架构概览

本项目遵循 **Backend Authority (后端权威)** 和 **Frontend Agnosticism (前端无关性)** 原则。

### 后端 (Rust)
后端是设备状态和业务逻辑的唯一真实来源。它使用 `inventory` crate 实现了模块化的插件系统：
*   **Controller Trait**: 抽象硬件设备，处理通信协议和虚拟布局映射。
*   **Effect Trait**: 抽象视觉效果，通过 `tick` 函数生成颜色数据。
*   **LightingManager**: 负责设备扫描、生命周期管理和灯效循环调度。

### 前端 (React)
前端是一个动态渲染器，负责展示状态和配置界面：
*   **动态参数渲染**: 根据后端返回的 `EffectParam` 元数据，自动生成滑块、选择框等控件。
*   **策略模式**: 使用 `ParamRenderer` 分发不同的 UI 组件。
*   **Chakra UI**: 使用 Chakra UI v3 组件库构建无障碍且美观的界面。

详细架构文档请参阅 [AGENTS.md](AGENTS.md)。

## 🛠️ 技术栈

### Frontend
*   **Framework**: React 19, Vite
*   **Language**: TypeScript
*   **UI Library**: Chakra UI v3, HeroUI, Lucide React
*   **Visualization**: PixiJS
*   **State/Animation**: Framer Motion

### Backend
*   **Core**: Rust, Tauri v2
*   **Plugin System**: `inventory`
*   **Hardware/IO**: `serialport`, `hidapi`, `mdns-sd`
*   **Audio/Video**: `cpal`, `spectrum-analyzer`, `screencapturekit` (macOS), `windows` crate (Windows), `xcap` (Linux)

## 🚀 快速开始

### 环境要求
*   **Node.js**: v18+
*   **Rust**: 最新稳定版
*   **包管理器**: pnpm (推荐) 或 npm/yarn

### 安装依赖

1.  克隆仓库：
    ```bash
    git clone https://github.com/Zayrick/Light.git
    cd Light
    ```

2.  安装前端依赖：
    ```bash
    pnpm install
    ```

3.  安装 Rust 依赖（通常在构建时自动处理，但需确保环境配置正确）：
    确保已安装 Tauri 前置依赖 (如 Windows 上的 WebView2, Linux 上的 webkit2gtk 等)。

### 运行开发环境

启动 Tauri 开发模式（同时启动前端和后端）：

```bash
pnpm tauri dev
```

## 💻 开发指南

### 添加新设备支持 (Controller)
1.  在 `src-tauri/src/resource/controller/` 下创建一个新模块。
2.  实现 `Controller` trait。
3.  定义 `ControllerMetadata` 并使用 `inventory::submit!` 注册。

### 添加新灯效 (Effect)
1.  在 `src-tauri/src/resource/effect/` 下创建一个新模块。
2.  实现 `Effect` trait。
3.  定义 `EffectMetadata` 和参数 `EffectParam`，并使用 `inventory::submit!` 注册。

无需修改核心逻辑代码，系统会自动发现新添加的组件。

## 📂 目录结构

```
Light/
├── src/                  # React 前端源码
│   ├── features/         # 业务功能模块 (Devices, Home)
│   ├── components/       # 通用 UI 组件
│   ├── services/         # Tauri API 通信层
│   └── styles/           # 全局样式和主题
├── src-tauri/            # Rust 后端源码
│   ├── src/
│   │   ├── api/          # Tauri Commands 和 DTOs
│   │   ├── interface/    # Traits 定义 (Controller, Effect)
│   │   ├── manager/      # 核心逻辑 (LightingManager)
│   │   └── resource/     # 插件实现 (Controllers, Effects)
│   └── Cargo.toml
├── AGENTS.md             # 详细架构设计文档
└── package.json
```

## 📄 许可证

[MIT License](LICENSE)
