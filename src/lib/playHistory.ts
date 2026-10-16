import { inTauri } from "@/lib/api";
import { recordPlay } from "@/lib/historyApi";
import { shouldRecordPlay } from "@/lib/playGate";
import { usePlayerStore } from "@/store/playerStore";

/** P10: app-lifetime play tracking — see playGate.ts for the pure rule. */
export function initPlayHistory(): () => void {
  if (!inTauri) return () => {};
  return usePlayerStore.subscribe((state, prev) => {
    if (shouldRecordPlay(prev, state)) {
      // Fire-and-forget: history is bookkeeping, never blocks playback.
      void recordPlay(state.currentTrackId!).catch(() => {});
    }
  });
}
