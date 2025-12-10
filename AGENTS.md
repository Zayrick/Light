# AI 代理指南

本文档概述了 Light 项目的架构决策和编码规范。
旨在指导未来的开发工作，并确保代码库的一致性。

## 架构概览

"Light" 项目是一个基于 **Tauri** (Rust 后端) 和 **React** (前端) 构建的跨平台 RGB 灯光控制应用程序。

### 核心理念
1.  **后端权威 (Backend Authority)**：Rust 后端是设备状态、灯效能力和业务逻辑的唯一真实来源（Single Source of Truth）。
2.  **前端无关性 (Frontend Agnosticism)**：前端是一个动态渲染器。它应该只需极少的更改即可支持新的后端功能（例如，新的灯效或设备）。
3.  **开闭原则 (Open/Closed Principle)**：系统设计为可扩展（添加新灯效/控制器），而无需修改核心逻辑。

---

## 后端设计 (Rust)

后端围绕模块化资源组织，并通过 `inventory` crate 进行去中心化注册。

### 1. 插件系统 (Inventory)
我们使用 `inventory` crate 在链接时自动收集 `Controller` 和 `Effect` 的实现。这允许一种类似插件的架构，添加新功能就像定义一个结构体并将其提交到 inventory 一样简单。

**如何添加新的控制器/灯效：**
-   **Controller**：实现 `Controller` trait 并将 `ControllerMetadata` 提交给 `inventory`。
-   **Effect**：实现 `Effect` trait 并将 `EffectMetadata` 提交给 `inventory`。
-   **无中心注册表**：你**不需要**修改中心的 `main.rs` 或 `registry.rs` 来注册新组件。

### 2. 抽象层 (Traits)

#### `Controller` Trait
抽象了照明设备的硬件细节。
-   **协议无关**：在内部处理特定的串口/HID 协议。
-   **虚拟布局**：将物理 LED 映射到虚拟 2D 坐标系 (`virtual_layout()`)。
-   **区域支持**：支持 `Single` (单个)、`Linear` (线性) 和 `Matrix` (矩阵) 区域，允许复杂的设备映射（例如，具有多个独立接头的主板）。

#### `Effect` Trait
抽象了视觉图案的生成。
-   **基于 Tick 更新**：实现 `tick(elapsed, buffer)` 来更新 LED 颜色。
-   **可参数化**：通过 `EffectParam` 定义其自己的配置参数。

### 3. LightingManager
`LightingManager` 充当中央协调器。
-   **设备扫描**：使用 inventory 探测所有已注册的控制器驱动程序。
-   **灯效执行**：管理活动的灯效循环并将更新分发到正确的控制器。

---

## 前端设计 (React)

前端设计为后端状态和能力的反映，采用 **后端驱动 UI (Backend-Driven UI)** 模式。

### 1. 后端驱动配置
前端不为特定灯效硬编码 UI 控件。相反，它根据后端返回的 `EffectParam` 列表动态渲染控件。

-   **定义链**：`EffectParam` (Rust) -> `EffectParamInfo` (JSON) -> `EffectParam` (TS)。
-   **支持类型**：`Slider` (滑块), `Select` (选择框) 等。

### 2. 渲染策略模式
`ParamRenderer.tsx` 充当分发器（策略模式），用于选择正确的 UI 组件。

-   **分发器**：`ParamRenderer` 根据 `param.type` 进行切换。
-   **隔离**：每种控件类型在 `src/features/devices/components/params/` 中都有专用的渲染器（例如 `SliderRenderer`, `SelectRenderer`）。
-   **可扩展性**：要添加新的控件类型（例如颜色选择器）：
    1.  在后端 `EffectParamKind` 中添加类型。
    2.  在前端创建 `ColorRenderer.tsx`。
    3.  更新 `ParamRenderer` 以处理新类型。

### 3. 动态依赖
UI 控件的可见性和启用状态由后端定义的规则管理。

-   **逻辑**：`checkDependency` (位于 `src/utils/effectUtils.ts`) 评估诸如 `equals` (等于)、`not_equals` (不等于) 等条件。
-   **流程**：
    1.  后端发送带有参数的 `dependency` (依赖) 规则。
    2.  前端 `DeviceDetail` 根据当前 `paramValues` 计算可见性。
    3.  `ParamRenderer` 接收 `disabled` 属性，或者如果隐藏则不挂载。

### 4. 通信层
所有 IPC 调用都封装在 `src/services/api.ts` 中。
-   **类型化接口**：提供强类型的异步函数（例如 `scanDevices`, `setEffect`），而不是原始的基于字符串的 `invoke` 调用。
-   **单点故障处理**：集中的 IPC 错误处理和日志记录。

---

## 编码规范

### 目录结构

#### 后端 (`src-tauri/src`)
-   `api/`: API 接口层。
    -   `commands.rs`: 所有 Tauri 命令 (`#[tauri::command]`) 的定义。
    -   `dto.rs`: 数据传输对象（DTO），用于前后端 JSON 序列化通信。
-   `interface/`: Trait 定义和共享数据结构。
-   `manager/`: 核心逻辑协调器 (`LightingManager`, `Inventory`)。
-   `resource/`: 控制器和灯效的实现（"插件"）。
    -   `controller/`: 硬件驱动程序。
    -   `effect/`: 视觉图案。

#### 前端 (`src`)
-   `features/`: 领域特定逻辑（垂直切片）。
    -   `devices/`: 设备列表、详情和配置。
    -   `home/`: 仪表板或着陆页视图。
-   `components/ui/`: 通用、可复用的 UI 组件（按钮、滑块），独立于业务逻辑。
-   `services/`: 外部通信 (Tauri API)。
-   `utils/`: 纯辅助函数。
-   `types/`: 类型定义。
    -   `device.ts`: 设备相关类型。
    -   `effect.ts`: 灯效相关类型。
    -   `index.ts`: 统一导出。

### 最佳实践
1.  **SOLID 原则**：
    -   **SRP (单一职责)**：保持组件小巧。`DeviceDetail` 处理布局，`ParamRenderer` 处理分发，`SliderRenderer` 处理滑动交互。
    -   **OCP (开闭原则)**：使用插件系统添加新的后端功能。
2.  **状态管理**：
    -   使用 React 本地状态处理瞬态 UI 交互（如拖动滑块）。
    -   仅在"已结算"事件（`onCommit`, `onChange`）时向后端提交，以防止 IPC 通道泛滥。
3.  **类型安全与代码组织**：
    -   确保 `src/types/` 下的定义严格匹配后端 `dto.rs` 和序列化的 Rust 结构体。
    -   保持入口文件（`lib.rs`, `index.ts`）简洁，将具体实现拆分到模块中（如 `api/commands.rs`, `api/dto.rs`）。
4.  **依赖管理**：
    -   安装外部库时候，请使用命令安装最新版本而不是直接修改package.json或者src-tauri\Cargo.toml文件。
5.  **样式与主题 (Styling & Theming)**：
    -   **统一颜色管理**：所有的颜色必须从 `src/styles/theme.css` 中定义的 CSS 变量获取，以确保统一的视觉风格。
    -   **双模适配**：禁止在组件代码中硬编码颜色值（Hex/RGB）。若需使用新颜色，必须先在 `theme.css` 中定义，并确保其在 **深色 (Dark)** 和 **浅色 (Light)** 模式下均有良好的视觉表现。
