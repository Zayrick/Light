import { ReactNode } from "react";
import { TitleBar } from "./TitleBar";
import { usePlatform } from "../../hooks/usePlatform";
import "../../styles/layout.css";

interface AppLayoutProps {
  sidebar: ReactNode;
  children: ReactNode;
  disableScroll?: boolean;
  hideScrollbar?: boolean;
}

export function AppLayout({
  sidebar,
  children,
  disableScroll = false,
  hideScrollbar = false,
}: AppLayoutProps) {
  const { isMacOS } = usePlatform();

  const scrollContainerClass = [
    "scroll-container",
    disableScroll ? "no-scroll" : "",
    hideScrollbar ? "no-scrollbar" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const contentClass = ["content-max-width", disableScroll ? "full-height" : ""]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={`app-layout ${isMacOS ? 'app-layout-macos' : ''}`}>
      <TitleBar />
      {sidebar}
      <main className="main-content">
        <div className={scrollContainerClass}>
          <div className={contentClass}>{children}</div>
        </div>
      </main>
    </div>
  );
}
