/** P6.4 View Transitions helper: morph the player-bar thumb ↔ hero art
    when the expanded Now Playing mounts/unmounts. The update runs inside
    `flushSync` — React must commit synchronously or the transition
    snapshots the pre-update DOM. Reduced motion (or an old WebView)
    falls back to an instant swap. */

import { flushSync } from "react-dom";

export function prefersReducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function viewTransitionSupported(): boolean {
  return typeof document.startViewTransition === "function";
}

export function runWithViewTransition(update: () => void): void {
  if (viewTransitionSupported() && !prefersReducedMotion()) {
    document.startViewTransition(() => flushSync(update));
  } else {
    update();
  }
}
