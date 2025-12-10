import { forwardRef } from "react";
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
  ({ className, ...props }, ref) => (
    <ArkTabs.List
      ref={ref}
      {...props}
      className={cx("ui-tabs__list", className)}
    />
  )
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

