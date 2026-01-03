import {
  forwardRef,
  useCallback,
  useRef,
  type MutableRefObject,
  type WheelEvent as ReactWheelEvent,
} from "react";
import {
  Tabs as ArkTabs,
  type TabContentProps,
  type TabIndicatorProps,
  type TabListProps,
  type TabsRootProps,
  type TabTriggerProps,
} from "@ark-ui/react/tabs";
import "./Tabs.css";

type WithClassName<T> = Omit<T, "className"> & { className?: string };

const cx = (...values: Array<string | undefined>) =>
  values.filter(Boolean).join(" ");

const Root = forwardRef<HTMLDivElement, WithClassName<TabsRootProps>>(
  ({ className, ...props }, ref) => (
    <ArkTabs.Root ref={ref} {...props} className={cx("ui-tabs", className)} />
  )
);
Root.displayName = "TabsRoot";

const List = forwardRef<HTMLDivElement, WithClassName<TabListProps>>(
  ({ className, onWheel, ...props }, ref) => {
    const elRef = useRef<HTMLDivElement | null>(null);

    const setRefs = useCallback(
      (node: HTMLDivElement | null) => {
        elRef.current = node;
        if (!ref) return;
        if (typeof ref === "function") {
          ref(node);
          return;
        }
        (ref as MutableRefObject<HTMLDivElement | null>).current = node;
      },
      [ref]
    );

    const handleWheel = useCallback(
      (e: ReactWheelEvent<HTMLDivElement>) => {
        // Let consumers run first (and potentially preventDefault)
        onWheel?.(e);
        if (e.defaultPrevented) return;

        // Only translate vertical wheel to horizontal scroll when:
        // - the list actually overflows horizontally
        // - user isn't already requesting horizontal scroll (shift)
        // - the gesture is primarily vertical (deltaY)
        if (e.shiftKey) return;
        if (Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;

        const el = elRef.current;
        if (!el) return;
        if (el.scrollWidth <= el.clientWidth) return;

        const maxLeft = el.scrollWidth - el.clientWidth;
        if (maxLeft <= 0) return;

        const prevLeft = el.scrollLeft;
        const nextLeft = Math.max(0, Math.min(maxLeft, prevLeft + e.deltaY));

        // Only prevent page scroll if we actually moved horizontally.
        if (nextLeft !== prevLeft) {
          el.scrollLeft = nextLeft;
          e.preventDefault();
        }
      },
      [onWheel]
    );

    return (
      <ArkTabs.List
        ref={setRefs}
        {...props}
        onWheel={handleWheel}
        className={cx("ui-tabs__list", className)}
      />
    );
  }
);
List.displayName = "TabsList";

const Trigger = forwardRef<
  HTMLButtonElement,
  WithClassName<TabTriggerProps>
>(({ className, ...props }, ref) => (
  <ArkTabs.Trigger
    ref={ref}
    {...props}
    className={cx("ui-tabs__trigger", className)}
  />
));
Trigger.displayName = "TabsTrigger";

const Indicator = forwardRef<HTMLDivElement, WithClassName<TabIndicatorProps>>(
  ({ className, ...props }, ref) => (
    <ArkTabs.Indicator
      ref={ref}
      {...props}
      className={cx("ui-tabs__indicator", className)}
    />
  )
);
Indicator.displayName = "TabsIndicator";

const Content = forwardRef<HTMLDivElement, WithClassName<TabContentProps>>(
  ({ className, ...props }, ref) => (
    <ArkTabs.Content
      ref={ref}
      {...props}
      className={cx("ui-tabs__content", className)}
    />
  )
);
Content.displayName = "TabsContent";

export const Tabs = {
  Root,
  List,
  Trigger,
  Indicator,
  Content,
};

