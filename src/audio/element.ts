/** Creates one wired engine <audio> element and its DOM event handlers
    (P3.3/3.4). Store reads/writes live here so PlayerEngine stays slim. */
import { advanceQueueAfterFade } from "./crossfade";
import { usePlayerStore } from "@/store/playerStore";

export type ElementHooks = {
  currentEl: () => HTMLAudioElement | null;
  isSuppressed: () => boolean;
  isFading: () => boolean;
  maybeBeginFade: () => void;
  onError: (el: HTMLAudioElement) => void;
};

function onTimeUpdate(el: HTMLAudioElement, hooks: ElementHooks): void {
  if (el !== hooks.currentEl() || hooks.isSuppressed()) return;
  const store = usePlayerStore.getState();
  if (Math.abs(el.currentTime - store.positionSec) >= 0.25) {
    store.setPosition(el.currentTime);
  }
  hooks.maybeBeginFade();
}

function onMeta(el: HTMLAudioElement, hooks: ElementHooks): void {
  if (el !== hooks.currentEl()) return;
  const duration = Number.isFinite(el.duration) ? el.duration : 0;
  usePlayerStore.getState().setDuration(duration);
}

function onEnded(el: HTMLAudioElement, hooks: ElementHooks): void {
  if (el !== hooks.currentEl() || hooks.isFading()) return;
  const store = usePlayerStore.getState();
  if (store.repeatMode === "track") {
    el.currentTime = 0;
    void el.play().catch(() => {});
    return;
  }
  advanceQueueAfterFade();
}

export function createEngineElement(hooks: ElementHooks): HTMLAudioElement {
  const el = new Audio();
  // Invariant 1: audio only via the loopback server, CORS-anonymous so the
  // AnalyserNode receives real data.
  el.crossOrigin = "anonymous";
  el.preload = "auto";

  el.addEventListener("ended", () => onEnded(el, hooks));
  el.addEventListener("timeupdate", () => onTimeUpdate(el, hooks));
  el.addEventListener("loadedmetadata", () => onMeta(el, hooks));
  el.addEventListener("error", () => hooks.onError(el));
  el.addEventListener("play", () => {
    if (!hooks.isSuppressed() && el === hooks.currentEl()) {
      usePlayerStore.getState().setPlaying(true);
    }
  });
  el.addEventListener("pause", () => {
    // Natural end-of-track fires 'pause' too — that path belongs to the
    // 'ended' handler (queue advance / crossfade swap), not a user pause.
    if (hooks.isSuppressed() || el !== hooks.currentEl() || el.ended) return;
    usePlayerStore.getState().setPlaying(false);
  });
  return el;
}
