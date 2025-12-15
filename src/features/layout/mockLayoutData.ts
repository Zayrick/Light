import { createTreeCollection } from "@ark-ui/react/tree-view";

export interface LayoutNode {
  id: string;
  name: string;
  children?: LayoutNode[];
}

export const LAYOUT_ROOT_ID = "layout-placeholder";

export const layoutCollection = createTreeCollection<LayoutNode>({
  nodeToValue: (node) => node.id,
  nodeToString: (node) => node.name,
  rootNode: {
    id: "ROOT",
    name: "",
    children: [
      {
        id: LAYOUT_ROOT_ID,
        name: "Layout Preview",
        children: [
          {
            id: "layout-keyboard",
            name: "Keyboard",
            children: [
              { id: "layout-keyboard-60", name: "60% Layout" },
              { id: "layout-keyboard-tkl", name: "TKL Layout" },
              { id: "layout-keyboard-full", name: "Full Size" },
            ],
          },
          {
            id: "layout-mouse",
            name: "Gaming Mouse",
          },
          {
            id: "layout-headset",
            name: "Headset Stand",
          },
          {
            id: "layout-strip",
            name: "LED Strip",
            children: [
              { id: "layout-strip-30", name: "30 LEDs/m" },
              { id: "layout-strip-60", name: "60 LEDs/m" },
              { id: "layout-strip-144", name: "144 LEDs/m" },
            ],
          },
          {
            id: "layout-matrix",
            name: "Matrix Panel",
            children: [
              { id: "layout-matrix-8x8", name: "8×8 Matrix" },
              { id: "layout-matrix-16x16", name: "16×16 Matrix" },
            ],
          },
          {
            id: "layout-ring",
            name: "Ring Light",
            children: [
              { id: "layout-ring-12", name: "12 LEDs" },
              { id: "layout-ring-24", name: "24 LEDs" },
            ],
          },
        ],
      },
    ],
  },
});

