import { HTMLAttributes, ReactNode } from "react";
import clsx from "clsx";
import "../../styles/layout.css";

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  hoverable?: boolean;
}

export function Card({ children, className, hoverable = false, style, ...props }: CardProps) {
  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!hoverable) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    e.currentTarget.style.setProperty("--mouse-x", `${x}px`);
    e.currentTarget.style.setProperty("--mouse-y", `${y}px`);
    e.currentTarget.style.setProperty("--spotlight-opacity", "1");
  };

  const handleMouseLeave = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!hoverable) return;
    e.currentTarget.style.setProperty("--spotlight-opacity", "0");
    if (props.onMouseLeave) {
      props.onMouseLeave(e);
    }
  };

  return (
    <div
      className={clsx("device-card", hoverable && "card-interactive", className)}
      style={{
        ...(hoverable ? { cursor: "pointer" } : {}),
        ...style,
      }}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
      {...props}
    >
      {children}
    </div>
  );
}
