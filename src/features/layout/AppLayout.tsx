import { ReactNode } from "react";
import { TitleBar } from "./TitleBar";
import "../../styles/layout.css";

interface AppLayoutProps {
  sidebar: ReactNode;
  children: ReactNode;
  disableScroll?: boolean;
}

export function AppLayout({ sidebar, children, disableScroll = false }: AppLayoutProps) {
  return (
    <div className="app-layout">
      <TitleBar />
      {sidebar}
      <main className="main-content">
        <div className={`scroll-container ${disableScroll ? 'no-scroll' : ''}`}>
          <div className={`content-max-width ${disableScroll ? 'full-height' : ''}`}>
            {children}
          </div>
        </div>
      </main>
    </div>
  );
}
