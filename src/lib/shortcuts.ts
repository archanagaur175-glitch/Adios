/** Global in-app keyboard shortcuts (P3.4 / architecture §2.3).
    Space toggle · ←/→ seek ±5 s · ↑/↓ volume ±5 % · M mute · Ctrl+K search.
    L toggles immersive lyrics (DESIGN §7); Q (queue) is reserved. */

import { usePlayerStore } from "@/store/playerStore";
import { useSearchStore } from "@/store/searchStore";
import { useUiStore } from "@/store/uiStore";

const SEEK_STEP_SEC = 5;
const VOLUME_STEP = 0.05;

function isEditable(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el?.tagName) return false;
  return el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable;
}

/** Install the document-level keydown handler; returns a cleanup function. */
export function installShortcuts(): () => void {
  const onKey = (e: KeyboardEvent) => {
    // Ctrl/Cmd+K toggles the search overlay from anywhere (even inputs).
    if (e.ctrlKey || e.metaKey) {
      if (e.key.toLowerCase() === "k") {
        e.preventDefault();
        const search = useSearchStore.getState();
        search.setOpen(!search.open);
      }
      return;
    }
    if (isEditable(e.target)) return;
    const store = usePlayerStore.getState();

    switch (e.key) {
      case " ": {
        // Let a focused button/link consume Space as its own activation.
        const tag = (e.target as HTMLElement | null)?.tagName;
        if (tag === "BUTTON" || tag === "A") return;
        if (store.queue.length === 0) return;
        e.preventDefault();
        store.setPlaying(!store.isPlaying);
        break;
      }
      case "ArrowLeft":
        e.preventDefault();
        store.seekTo(store.positionSec - SEEK_STEP_SEC);
        break;
      case "ArrowRight":
        e.preventDefault();
        store.seekTo(store.positionSec + SEEK_STEP_SEC);
        break;
      case "ArrowUp":
        e.preventDefault();
        store.setVolume(store.volume + VOLUME_STEP);
        break;
      case "ArrowDown":
        e.preventDefault();
        store.setVolume(store.volume - VOLUME_STEP);
        break;
      case "m":
      case "M":
        store.setMuted(!store.muted);
        break;
      case "l":
      case "L": {
        const ui = useUiStore.getState();
        ui.setLyricsOpen(!ui.lyricsOpen);
        break;
      }
    }
  };

  document.addEventListener("keydown", onKey);
  return () => document.removeEventListener("keydown", onKey);
}
