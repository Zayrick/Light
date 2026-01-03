import { TreeView, createTreeCollection } from "@ark-ui/react/tree-view";
import { Menu } from "@ark-ui/react/menu";
import { ChevronRight, PlugZap, Zap, Power, Settings } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import clsx from "clsx";
import { useMemo } from "react";
import type { Device, ScopeModeState } from "../../types";
import type { SelectedScope } from "../../hooks/useDevices";
import { 
  HIGHLIGHT_TRANSITION, 
  BRANCH_TRANSITION,
  branchContentVariants 
} from "../../motion/transitions";

type ControlState = "none" | "explicit" | "inherited";

function DeviceContextMenu({ children }: { children: React.ReactNode }) {
  return (
    <Menu.Root lazyMount unmountOnExit>
      <Menu.ContextTrigger asChild>{children}</Menu.ContextTrigger>
      <Menu.Positioner>
        <Menu.Content>
          <Menu.Item value="turn-off">
            <Power />
            关灯
          </Menu.Item>
          <Menu.Item value="settings">
            <Settings />
            设置设备
          </Menu.Item>
        </Menu.Content>
      </Menu.Positioner>
    </Menu.Root>
  );
}

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
  return devices.map((d) => {
    // Single-child compression:
    // If a device has exactly one output, we merge the device and output nodes.
    // The node visually represents the device (name, icon, port),
    // but functionally acts as the output (id, selection, control state).
    if (d.outputs.length === 1) {
      const o = d.outputs[0];
      return {
        id: `out:${d.port}:${o.id}`, // Use output ID so selection works for the output scope
        name: d.model, // Use device name
        kind: "device", // Visual style: Device
        port: d.port,
        outputId: o.id, // Functional scope: Output
        controlState: controlStateFromMode(o.mode), // Status from output
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
      };
    }

    return {
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
    };
  });
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
    // Expand the path to the selected scope.
    // Note: segment is a leaf node, so it does not need to be in expandedValue.
    const values: string[] = [`dev:${selectedScope.port}`];
    if (selectedScope.outputId) {
      values.push(`out:${selectedScope.port}:${selectedScope.outputId}`);
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
            expandedValue={expandedValue}
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
  expandedValue: string[];
}

const DeviceTreeItem = ({
  node,
  indexPath,
  selectedNodeId,
  onSelectScope,
  onMouseMove,
  onMouseLeave,
  expandedValue,
}: DeviceTreeItemProps) => {
  const isSelected = selectedNodeId === node.id;
  const depth = indexPath.length;
  const isExpanded = expandedValue.includes(node.id);

  const indent = depth === 2 ? 8 : depth >= 3 ? 20 : 0;

  // Branch controls (device / output nodes with children) should not be indented at root.
  const branchIndentStyle = depth > 1 ? { marginLeft: `${indent}px` } : undefined;

  // TreeView.Item has a default CSS margin-left (see `.layout-tree-item`) which makes
  // root leaf nodes look like "children". We always set margin-left explicitly here.
  const itemIndentStyle = { marginLeft: `${indent}px` };

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

  const Highlight = isSelected ? (
    <motion.div
      layoutId="sidebar-active-highlight"
      className="active-highlight"
      transition={HIGHLIGHT_TRANSITION}
      style={{ zIndex: -1 }}
    />
  ) : null;

  const handleClick = () => {
    onSelectScope({
      port: node.port,
      outputId: node.outputId,
      segmentId: node.segmentId,
    });
  };

  return (
    <TreeView.NodeProvider node={node} indexPath={indexPath}>
      {node.children ? (
        <TreeView.Branch className="layout-tree-branch">
          <DeviceContextMenu>
            <motion.div layout>
              <TreeView.BranchControl
                className={clsx("device-list-item layout-branch-control", isSelected && "active")}
                style={branchIndentStyle}
                onClick={handleClick}
                onMouseMove={onMouseMove}
                onMouseLeave={onMouseLeave}
              >
                {Highlight}
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
            </motion.div>
          </DeviceContextMenu>
          {/* Use AnimatePresence + motion.div for smooth height animation */}
          <AnimatePresence initial={false}>
            {isExpanded && (
              <motion.div
                key={`branch-content-${node.id}`}
                variants={branchContentVariants}
                initial="collapsed"
                animate="expanded"
                exit="collapsed"
                transition={BRANCH_TRANSITION}
                style={{ overflow: "hidden" }}
              >
                <TreeView.BranchContent className="layout-branch-content-inner">
                  {node.children.map((child, index) => (
                    <DeviceTreeItem
                      key={child.id}
                      node={child}
                      indexPath={[...indexPath, index]}
                      selectedNodeId={selectedNodeId}
                      onSelectScope={onSelectScope}
                      onMouseMove={onMouseMove}
                      onMouseLeave={onMouseLeave}
                      expandedValue={expandedValue}
                    />
                  ))}
                </TreeView.BranchContent>
              </motion.div>
            )}
          </AnimatePresence>
        </TreeView.Branch>
      ) : (
        <DeviceContextMenu>
          <motion.div layout>
            <TreeView.Item
              className={clsx(
                "device-list-item",
                node.kind === "device" ? "layout-branch-control" : "layout-tree-item",
                isSelected && "active"
              )}
              style={itemIndentStyle}
              onClick={handleClick}
              onMouseMove={onMouseMove}
              onMouseLeave={onMouseLeave}
            >
              {Highlight}
              {node.kind === "device" ? (
                <>
                  {icon}
                  <div className="device-list-info">
                    <div className="device-list-item-name">{node.name}</div>
                    <div className="device-list-item-port">{node.port}</div>
                  </div>
                </>
              ) : (
                <TreeView.ItemText className="layout-item-text">
                  {segmentDot ?? icon}
                  <span className="device-list-item-name">{node.name}</span>
                </TreeView.ItemText>
              )}
            </TreeView.Item>
          </motion.div>
        </DeviceContextMenu>
      )}
    </TreeView.NodeProvider>
  );
};


