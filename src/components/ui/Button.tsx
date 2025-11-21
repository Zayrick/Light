import { ButtonHTMLAttributes, ReactNode } from "react";
import clsx from "clsx";
import "../../styles/layout.css"; // Import styles to ensure they are available

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "icon";
  isLoading?: boolean;
  icon?: ReactNode;
  children?: ReactNode;
}

export function Button({
  variant = "primary",
  isLoading,
  icon,
  children,
  className,
  disabled,
  ...props
}: ButtonProps) {
  return (
    <button
      className={clsx("btn", `btn-${variant}`, className)}
      disabled={isLoading || disabled}
      {...props}
    >
      {isLoading ? (
        <span className="animate-spin">↻</span> 
      ) : (
        icon
      )}
      {children}
    </button>
  );
}

