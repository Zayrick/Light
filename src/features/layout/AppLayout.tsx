import { ReactNode } from "react";
import { TitleBar } from "./TitleBar";
import "../../styles/layout.css";

interface AppLayoutProps {
  sidebar: ReactNode;
  children: ReactNode;
}

export function AppLayout({ sidebar, children }: AppLayoutProps) {
  return (
    <div className="app-layout">
      <TitleBar />
      {sidebar}
      <main className="main-content">
        <div className="scroll-container">
          <div className="content-max-width">
            {children}
          </div>
        </div>
      </main>
    </div>
  );
}

