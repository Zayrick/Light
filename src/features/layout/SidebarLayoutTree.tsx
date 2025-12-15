import { TreeView } from "@ark-ui/react/tree-view";
import { ChevronRight, LayoutGrid } from "lucide-react";
import { AnimatePresence, motion } from "framer-motion";
import clsx from "clsx";
import { useMemo } from "react";
import { LayoutNode, layoutCollection, LAYOUT_ROOT_ID } from "./mockLayoutData";
import { HIGHLIGHT_TRANSITION } from "./constants";

interface SidebarLayoutTreeProps {
  activeTab: string;
  selectedLayoutId: string | null;
  onSelect: (id: string) => void;
  onMouseMove: (e: React.MouseEvent<HTMLDivElement>) => void;
  onMouseLeave: (e: React.MouseEvent<HTMLDivElement>) => void;
}

// 查找节点的父链（用于展开选中节点的所有父级）
const findAncestorIds = (
  node: LayoutNode,
  targetId: string,
  path: string[] = []
): string[] | null => {
  if (node.id === targetId) {
    return path;
  }
  if (node.children) {
    for (const child of node.children) {
      const result = findAncestorIds(child, targetId, [...path, node.id]);
      if (result) return result;
    }
  }
  return null;
};

export function SidebarLayoutTree({
  activeTab,
  selectedLayoutId,
  onSelect,
  onMouseMove,
  onMouseLeave,
}: SidebarLayoutTreeProps) {
  // Auto-expand logic:
  // - 选中 Layout Preview 时，展开 Layout Preview
  // - 选中子设备时，展开 Layout Preview + 该子设备
  // - 选中子子设备时，展开 Layout Preview + 父子设备
  const expandedValue = useMemo(() => {
    let result: string[] = [];
    if (activeTab === "layout-preview" && selectedLayoutId) {
      // 始终展开根节点
      result = [LAYOUT_ROOT_ID];

      // 查找选中节点的祖先链并展开
      const rootNode = layoutCollection.rootNode.children?.[0];
      if (rootNode) {
        const ancestors = findAncestorIds(rootNode, selectedLayoutId, []);
        if (ancestors) {
          result = [...result, ...ancestors.filter((id) => id !== "ROOT")];
        }
        // 如果选中的是一个分支节点，也展开它
        if (selectedLayoutId !== LAYOUT_ROOT_ID) {
          result.push(selectedLayoutId);
        }
      }
    }
    return result;
  }, [activeTab, selectedLayoutId]);

  return (
    <TreeView.Root
      collection={layoutCollection}
      className="layout-tree-view"
      expandedValue={expandedValue}
    >
      <TreeView.Tree>
        {layoutCollection.rootNode.children?.map((node, index) => (
          <LayoutTreeNode
            key={node.id}
            node={node}
            indexPath={[index]}
            selectedLayoutId={activeTab === "layout-preview" ? selectedLayoutId : null}
            onSelect={onSelect}
            onMouseMove={onMouseMove}
            onMouseLeave={onMouseLeave}
          />
        ))}
      </TreeView.Tree>
    </TreeView.Root>
  );
}

interface LayoutTreeNodeProps extends TreeView.NodeProviderProps<LayoutNode> {
  selectedLayoutId: string | null;
  onSelect: (id: string) => void;
  onMouseMove: (e: React.MouseEvent<HTMLDivElement>) => void;
  onMouseLeave: (e: React.MouseEvent<HTMLDivElement>) => void;
}

const LayoutTreeNode = ({
  node,
  indexPath,
  selectedLayoutId,
  onSelect,
  onMouseMove,
  onMouseLeave,
}: LayoutTreeNodeProps) => {
  const isSelected = selectedLayoutId === node.id;
  const depth = indexPath.length;
  // depth 1=Layout Preview, 2=Keyboard等 显示图标; depth 3+=子子设备不显示图标
  const showIcon = depth <= 2;

  // Render Highlight background
  const Highlight = () => (
    <AnimatePresence>
      {isSelected && (
        <motion.div
          layoutId="active-nav"
          className="active-highlight"
          transition={HIGHLIGHT_TRANSITION}
        />
      )}
    </AnimatePresence>
  );

  // 根据深度计算缩进: depth 2 (子设备) 缩进 8px, depth 3 (子子设备) 缩进 20px
  const getIndent = () => {
    if (depth === 2) return 8;
    if (depth >= 3) return 20;
    return 0;
  };
  const indentStyle = depth > 1 ? { marginLeft: `${getIndent()}px` } : undefined;

  return (
    <TreeView.NodeProvider key={node.id} node={node} indexPath={indexPath}>
      {node.children ? (
        <TreeView.Branch className="layout-tree-branch">
          <TreeView.BranchControl
            className={clsx("device-list-item layout-branch-control", isSelected && "active")}
            style={indentStyle}
            onClick={() => onSelect(node.id)}
            onMouseMove={onMouseMove}
            onMouseLeave={onMouseLeave}
          >
            <Highlight />
            {showIcon && <LayoutGrid size={18} className="device-list-icon" />}
            <div className="device-list-info">
              <div className="device-list-item-name">{node.name}</div>
              {depth === 1 && <div className="device-list-item-port">Virtual Device</div>}
            </div>
            <TreeView.BranchIndicator className="layout-branch-indicator">
              <ChevronRight size={14} />
            </TreeView.BranchIndicator>
          </TreeView.BranchControl>
          <TreeView.BranchContent className="layout-branch-content">
            {node.children.map((child, index) => (
              <LayoutTreeNode
                key={child.id}
                node={child}
                indexPath={[...indexPath, index]}
                selectedLayoutId={selectedLayoutId}
                onSelect={onSelect}
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
          onClick={() => onSelect(node.id)}
          onMouseMove={onMouseMove}
          onMouseLeave={onMouseLeave}
        >
          <Highlight />
          <TreeView.ItemText className="layout-item-text">
            {showIcon && <LayoutGrid size={16} className="device-list-icon layout-child-icon" />}
            <span className="device-list-item-name">{node.name}</span>
          </TreeView.ItemText>
        </TreeView.Item>
      )}
    </TreeView.NodeProvider>
  );
};

