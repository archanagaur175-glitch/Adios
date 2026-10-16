import { getSettings, setSetting } from "@/lib/api";
import { loadQueue, saveQueue } from "@/lib/sessionApi";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";

/** Session persistence (P8.3): queue via `session_*` commands, the rest via
    generic settings keys. Restore NEVER autoplays — queue + seek, paused. */

const SAVE_DEBOUNCE_MS = 500;
const POSITION_MIN_INTERVAL_MS = 3000;

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));

let saveTimer: ReturnType<typeof setTimeout> | null = null;
let lastPositionSave = 0;

/** Persist the current player state immediately (also cancels a pending save). */
export async function flushSession(): Promise<void> {
  if (saveTimer) {
    clearTimeout(saveTimer);
    saveTimer = null;
  }
  await flush();
}

async function flush(): Promise<void> {
  const s = usePlayerStore.getState();
  lastPositionSave = Date.now();
  try {
    await saveQueue(s.queue, s.queueIndex);
    await setSetting("sessionPositionSec", s.positionSec);
    await setSetting("volume", s.volume);
    await setSetting("muted", s.muted);
  } catch (err) {
    console.warn("[session] save failed:", err);
  }
}

function schedule(withPosition: boolean): void {
  if (!withPosition) {
    // Position rides along whenever a structural save happens anyway.
    lastPositionSave = 0;
  }
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveTimer = null;
    void flush();
  }, SAVE_DEBOUNCE_MS);
}

/** Subscribe to player changes; returns an unsubscribe (StrictMode-safe). */
export function attachSessionPersist(): () => void {
  return usePlayerStore.subscribe((state, prev) => {
    if (
      state.queue !== prev.queue ||
      state.queueIndex !== prev.queueIndex ||
      state.volume !== prev.volume ||
      state.muted !== prev.muted
    ) {
      schedule(false);
      return;
    }
    if (
      state.positionSec !== prev.positionSec &&
      Date.now() - lastPositionSave >= POSITION_MIN_INTERVAL_MS
    ) {
      schedule(true);
    }
  });
}

/** Boot restore: apply volume/muted + queue + position, paused. */
export async function restoreSession(): Promise<void> {
  let stored: Record<string, unknown>;
  try {
    stored = await getSettings();
  } catch {
    return;
  }
  const volume = typeof stored.volume === "number" ? clamp01(stored.volume) : null;
  const muted = stored.muted === true;
  const positionSec =
    typeof stored.sessionPositionSec === "number" && stored.sessionPositionSec > 0
      ? stored.sessionPositionSec
      : 0;

  const patch: { volume?: number; muted?: boolean } = {};
  if (volume !== null) patch.volume = volume;
  if (muted) patch.muted = true;
  if (Object.keys(patch).length > 0) usePlayerStore.setState(patch);

  const snap = await loadQueue();
  if (!snap || snap.trackIds.length === 0) return;

  const known = new Set(useLibraryStore.getState().tracks.map((t) => t.id));
  const ids = snap.trackIds.filter((id) => known.has(id));
  if (ids.length === 0) return;

  const queueIndex = Math.min(Math.max(snap.queueIndex, 0), ids.length - 1);
  usePlayerStore.getState().setQueue(ids, queueIndex);
  if (positionSec > 0) usePlayerStore.getState().seekTo(positionSec);

  const missing = snap.trackIds.length - ids.length;
  if (snap.dropped > 0 || missing > 0) {
    useUiStore.getState().pushToast({
      message: `Queue updated — ${snap.dropped + missing} unavailable track(s) skipped`,
      tone: "info",
    });
  }
}
