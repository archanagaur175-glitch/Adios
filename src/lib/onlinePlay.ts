import { resolveQuery } from "@/lib/resolverApi";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";

/** P9.5: play a MusicBrainz search hit that is not in the library.
    `resolveQuery` runs the proven P4.4 chain (yt-dlp search → loopback
    proxy, Piped fallback, re-resolve on 403); the candidate becomes a
    transient stream track (negative id, in-memory only) and plays. */

export interface OnlinePick {
  mbid: string;
  title: string;
  artist?: string;
  durationMs?: number;
}

/** Returns true when playback started (callers close the overlay). */
export async function playOnlinePick(pick: OnlinePick): Promise<boolean> {
  try {
    const candidate = await resolveQuery({
      artist: pick.artist ?? "",
      title: pick.title,
      durationMs: pick.durationMs ?? null,
    });
    if (!candidate) {
      useUiStore.getState().pushToast({
        message: `No stream found for “${pick.title}”`,
        tone: "info",
      });
      return false;
    }
    const id = useLibraryStore.getState().upsertStreamTrack({
      source: "stream",
      url: candidate.urlOrPath,
      artist: pick.artist ?? "",
      title: pick.title,
      durationMs: pick.durationMs ?? null,
      mbRecordingId: pick.mbid,
      addedAt: new Date().toISOString(),
    });
    usePlayerStore.getState().playQueue([id], 0);
    return true;
  } catch (err) {
    console.error("[search] resolve_query failed:", err);
    useUiStore.getState().pushToast({
      message: `Could not resolve “${pick.title}”`,
      tone: "info",
    });
    return false;
  }
}
