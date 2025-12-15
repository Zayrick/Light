import { TreeView, createTreeCollection } from "@ark-ui/react/tree-view";
import { ChevronRight, PlugZap, Zap } from "lucide-react";
import { AnimatePresence, motion } from "framer-motion";
import clsx from "clsx";
import { useMemo } from "react";
import type { Device, ScopeModeState } from "../../types";
import type { SelectedScope } from "../../hooks/useDevices";
import { HIGHLIGHT_TRANSITION } from "./constants";

type ControlState = "none" | "explicit" | "inherited";

interface DeviceTreeNode {
  id: string;
  name: string;
  kind: "device" | "output" | "segment";
  port: string;
  outputId?: string;
  segmentId?: string;
  controlState: ControlState;
  children?: DeviceTreeNode[];
}

function nodeIdForScope(scope: SelectedScope): string {
  if (scope.outputId && scope.segmentId) {
    return `seg:${scope.port}:${scope.outputId}:${scope.segmentId}`;
  }
  if (scope.outputId) {
    return `out:${scope.port}:${scope.outputId}`;
  }
  return `dev:${scope.port}`;
}

function controlStateFromMode(mode: ScopeModeState): ControlState {
  if (!mode.effective_effect_id) return "none";
  if (mode.selected_effect_id) return "explicit";
  return "inherited";
}

function buildTree(devices: Device[]): DeviceTreeNode[] {
  return devices.map((d) => ({
    id: `dev:${d.port}`,
    name: d.model,
    kind: "device",
    port: d.port,
    controlState: controlStateFromMode(d.mode),
    children: d.outputs.map((o) => ({
      id: `out:${d.port}:${o.id}`,
      name: o.name,
      kind: "output",
      port: d.port,
      outputId: o.id,
      controlState: controlStateFromMode(o.mode),
      // Segments are user-defined and only apply to linear outputs (future feature).
      children:
        o.output_type === "Linear" && o.segments.length > 0
          ? o.segments.map((s) => ({
              id: `seg:${d.port}:${o.id}:${s.id}`,
              name: s.name,
              kind: "segment",
              port: d.port,
              outputId: o.id,
              segmentId: s.id,
              controlState: controlStateFromMode(s.mode),
            }))
          : undefined,
    })),
  }));
}

interface SidebarDeviceTreeProps {
  activeTab: string;
  devices: Device[];
  selectedScope: SelectedScope | null;
  onSelectScope: (scope: SelectedScope) => void;
  onMouseMove: (e: React.MouseEvent<HTMLDivElement>) => void;
  onMouseLeave: (e: React.MouseEvent<HTMLDivElement>) => void;
}

export function SidebarDeviceTree({
  activeTab,
  devices,
  selectedScope,
  onSelectScope,
  onMouseMove,
  onMouseLeave,
}: SidebarDeviceTreeProps) {
  const nodes = useMemo(() => buildTree(devices), [devices]);

  const collection = useMemo(
    () =>
      createTreeCollection<DeviceTreeNode>({
        nodeToValue: (node) => node.id,
        nodeToString: (node) => node.name,
        rootNode: {
          id: "ROOT",
          name: "",
          kind: "device",
          port: "ROOT",
          controlState: "none",
          children: nodes,
        },
      }),
    [nodes]
  );

  const selectedNodeId = selectedScope ? nodeIdForScope(selectedScope) : null;

  const expandedValue = useMemo(() => {
    if (activeTab !== "device-detail" || !selectedScope) return [];
    const values: string[] = [];
    values.push(`dev:${selectedScope.port}`);
    if (selectedScope.outputId) values.push(`out:${selectedScope.port}:${selectedScope.outputId}`);
    if (selectedScope.segmentId && selectedScope.outputId) {
      values.push(`seg:${selectedScope.port}:${selectedScope.outputId}:${selectedScope.segmentId}`);
    }
    return values;
  }, [activeTab, selectedScope]);

  return (
    <TreeView.Root
      collection={collection}
      className="layout-tree-view"
      expandedValue={expandedValue}
    >
      <TreeView.Tree>
        {collection.rootNode.children?.map((node, index) => (
          <DeviceTreeItem
            key={node.id}
            node={node}
            indexPath={[index]}
            selectedNodeId={activeTab === "device-detail" ? selectedNodeId : null}
            onSelectScope={onSelectScope}
            onMouseMove={onMouseMove}
            onMouseLeave={onMouseLeave}
          />
        ))}
      </TreeView.Tree>
    </TreeView.Root>
  );
}

interface DeviceTreeItemProps extends TreeView.NodeProviderProps<DeviceTreeNode> {
  selectedNodeId: string | null;
  onSelectScope: (scope: SelectedScope) => void;
  onMouseMove: (e: React.MouseEvent<HTMLDivElement>) => void;
  onMouseLeave: (e: React.MouseEvent<HTMLDivElement>) => void;
}

const DeviceTreeItem = ({
  node,
  indexPath,
  selectedNodeId,
  onSelectScope,
  onMouseMove,
  onMouseLeave,
}: DeviceTreeItemProps) => {
  const isSelected = selectedNodeId === node.id;
  const depth = indexPath.length;

  const getIndent = () => {
    if (depth === 2) return 8;
    if (depth >= 3) return 20;
    return 0;
  };
  const indentStyle = depth > 1 ? { marginLeft: `${getIndent()}px` } : undefined;

  const indicatorColor =
    node.controlState === "explicit"
      ? "var(--success-color)"
      : node.controlState === "inherited"
        ? "var(--color-blue)"
        : "var(--color-gray)";

  const icon =
    node.kind === "device" ? (
      <Zap size={18} className="device-list-icon" style={{ color: indicatorColor }} />
    ) : node.kind === "output" ? (
      <PlugZap size={16} className="device-list-icon" style={{ color: indicatorColor }} />
    ) : null;

  const segmentDot =
    node.kind === "segment" ? (
      <span className="sidebar-status-dot-wrap" aria-hidden="true">
        <span className="sidebar-status-dot" style={{ backgroundColor: indicatorColor }} />
      </span>
    ) : null;

  const Highlight = () => (
    <AnimatePresence>
      {isSelected && (
        <motion.div
          layoutId="active-device-node"
          className="active-highlight"
          transition={HIGHLIGHT_TRANSITION}
        />
      )}
    </AnimatePresence>
  );

  const handleClick = () => {
    onSelectScope({
      port: node.port,
      outputId: node.outputId,
      segmentId: node.segmentId,
    });
  };

  return (
    <TreeView.NodeProvider key={node.id} node={node} indexPath={indexPath}>
      {node.children ? (
        <TreeView.Branch className="layout-tree-branch">
          <TreeView.BranchControl
            className={clsx("device-list-item layout-branch-control", isSelected && "active")}
            style={indentStyle}
            onClick={handleClick}
            onMouseMove={onMouseMove}
            onMouseLeave={onMouseLeave}
          >
            <Highlight />
            {icon}
            <div className="device-list-info">
              <div className="device-list-item-name">{node.name}</div>
              {node.kind === "device" && (
                <div className="device-list-item-port">{node.port}</div>
              )}
            </div>
            <TreeView.BranchIndicator className="layout-branch-indicator">
              <ChevronRight size={14} />
            </TreeView.BranchIndicator>
          </TreeView.BranchControl>
          <TreeView.BranchContent className="layout-branch-content">
            {node.children.map((child, index) => (
              <DeviceTreeItem
                key={child.id}
                node={child}
                indexPath={[...indexPath, index]}
                selectedNodeId={selectedNodeId}
                onSelectScope={onSelectScope}
                onMouseMove={onMouseMove}
                onMouseLeave={onMouseLeave}
              />
            ))}
          </TreeView.BranchContent>
        </TreeView.Branch>
      ) : (
        <TreeView.Item
          className={clsx("device-list-item layout-tree-item", isSelected && "active")}
          style={indentStyle}
          onClick={handleClick}
          onMouseMove={onMouseMove}
          onMouseLeave={onMouseLeave}
        >
          <Highlight />
          <TreeView.ItemText className="layout-item-text">
            {segmentDot ?? icon}
            <span className="device-list-item-name">{node.name}</span>
          </TreeView.ItemText>
        </TreeView.Item>
      )}
    </TreeView.NodeProvider>
  );
};


