import { HTMLAttributes, ReactNode } from "react";
import clsx from "clsx";
import "../../styles/layout.css";

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  hoverable?: boolean;
}

export function Card({ children, className, hoverable = false, ...props }: CardProps) {
  return (
    <div
      className={clsx("device-card", className)} // reusing device-card class for now, maybe rename in css later
      style={hoverable ? { cursor: "pointer" } : undefined}
      {...props}
    >
      {children}
    </div>
  );
}

