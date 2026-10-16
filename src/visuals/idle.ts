/** P6.4 ambient idle mode (DESIGN §6): while Now Playing is expanded,
    15 s without pointer/keyboard input adds `ambient-idle` on :root —
    chrome fades out, aurora + visualizer scale up ~8 % (pure CSS from
    that class). ANY input removes it instantly (restore animates back at
    --dur-base). No React re-renders; class + timer only. */

import { useUiStore } from "@/store/uiStore";

export const IDLE_MS = 15_000;

const INPUT_EVENTS = [
  "pointermove",
  "pointerdown",
  "keydown",
  "wheel",
  "touchstart",
] as const;

export function initAmbientIdle(): () => void {
  let timer = 0;

  const clear = () => {
    if (timer) {
      window.clearTimeout(timer);
      timer = 0;
    }
    document.documentElement.classList.remove("ambient-idle");
  };

  const arm = () => {
    clear();
    if (!useUiStore.getState().nowPlayingOpen) return;
    timer = window.setTimeout(() => {
      document.documentElement.classList.add("ambient-idle");
      timer = 0;
    }, IDLE_MS);
  };

  const unsub = useUiStore.subscribe((state, prev) => {
    if (state.nowPlayingOpen !== prev.nowPlayingOpen) arm();
  });

  const onInput = () => {
    if (document.documentElement.classList.contains("ambient-idle")) clear();
    else if (useUiStore.getState().nowPlayingOpen) arm();
  };
  for (const ev of INPUT_EVENTS) document.addEventListener(ev, onInput, { passive: true });
  arm(); // reconcile with a persisted/open state

  return () => {
    unsub();
    for (const ev of INPUT_EVENTS) document.removeEventListener(ev, onInput);
    clear();
  };
}
