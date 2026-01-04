/**
 * Unified motion tokens used by this project.
 * Keep this file intentionally small: only export what is actually used.
 */

import type { Transition, Variants } from "framer-motion";

// Shared ease curve (matches previous behavior)
const EASE_OUT_EXPO = [0.16, 1, 0.3, 1] as const;

// Navigation micro-interaction
export const NAV_TRANSITION: Transition = {
  duration: 0.25,
  ease: EASE_OUT_EXPO,
};

// Shared layout highlight (spring feels better than duration-based ease)
export const HIGHLIGHT_TRANSITION: Transition = {
  type: "spring",
  stiffness: 500,
  damping: 35,
  mass: 1,
};

// Page transitions
export const PAGE_TRANSITION: Transition = {
  duration: 0.35,
  ease: EASE_OUT_EXPO,
};

export const pageVariants: Variants = {
  enter: (direction: number) => ({
    y: direction > 0 ? 20 : -20,
    opacity: 0,
  }),
  center: { y: 0, opacity: 1 },
  exit: (direction: number) => ({
    y: direction > 0 ? -20 : 20,
    opacity: 0,
  }),
};

// Tree branch expand/collapse (height auto)
export const BRANCH_TRANSITION: Transition = {
  type: "tween",
  duration: 0.28,
  ease: EASE_OUT_EXPO,
};

export const branchContentVariants: Variants = {
  collapsed: { height: 0, opacity: 0 },
  expanded: { height: "auto", opacity: 1 },
};
