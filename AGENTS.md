# AI Agent Guidelines

This document outlines the architectural decisions and coding standards for the Light project.

## Architecture: Backend-Driven UI

The application follows a **Backend-Driven UI** pattern for device effect configuration.

### Core Principles

1.  **Backend Authority**: The Rust backend is the single source of truth for what effects exist and what parameters they require.
2.  **Frontend Agnosticism**: The frontend (React) should not hardcode UI for specific effects (e.g., "Rainbow" or "Breathing"). Instead, it renders generic controls based on the parameter types (Slider, Select, etc.) returned by the backend.
3.  **Open/Closed**: To add a new effect, you should only need to modify the Rust backend. The frontend should automatically adapt. To add a new *type* of control (e.g., Color Picker), you modify the frontend.

### Implementation Details

#### Parameter Rendering

-   **Dispatcher Pattern**: `ParamRenderer.tsx` acts as a dispatcher. It takes a raw `EffectParam` object and decides which specific component to render based on `param.type`.
-   **Isolation**: Each parameter type has its own renderer (e.g., `SliderRenderer.tsx`, `SelectRenderer.tsx`) located in `src/features/devices/components/params/`.
-   **Extension**: When the backend introduces a new parameter type (e.g., `type: 'color'`):
    1.  Create `ColorRenderer.tsx` in the `params/` directory.
    2.  Update `ParamRenderer.tsx` to import and use the new renderer in the switch statement.

#### State Management

-   **Transient State**: The frontend maintains transient state for UI responsiveness (e.g., dragging a slider).
-   **Commit Strategy**: Changes are committed to the backend only when necessary (e.g., `onCommit` for sliders, `onChange` for selects) to minimize IPC traffic.
-   **Dependencies**: Visibility logic (`isDependencySatisfied`) is calculated in the parent container (`DeviceDetail`) or a hook, but the *result* (visible/disabled) is passed down to the renderer.

## Coding Standards

### Directory Structure

-   `src/features/devices/components/params/`: Contains all atomic parameter renderers.
-   `src/components/ui/`: Contains generic, reusable UI components (not tied to business logic).

### Best Practices

-   **Clean Components**: Keep the main container (`DeviceDetail.tsx`) focused on layout and state orchestration. Delegate rendering details to sub-components.
-   **Type Safety**: Ensure all backend types (in `types/index.ts`) match the Rust struct definitions.
