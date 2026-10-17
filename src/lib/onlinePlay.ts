import { addStreamTrack } from "@/lib/onlineApi";
import { identityId, pickKey, type OnlinePick } from "@/lib/onlinePick";
import { resolveQuery, type Candidate } from "@/lib/resolverApi";
import { addPlaylistTrack } from "@/lib/playlistApi";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";
import type { Track } from "@/lib/types";

/** P9.5/P9.7: play an iTunes or MusicBrainz search hit. `resolveQuery` runs
    the proven P4.4 chain (yt-dlp ytsearch3 → loopback proxy, Piped fallback,
    re-resolve on 403); the candidate is persisted as a real 'stream' library
    row via add_stream_track (survives restarts, FTS-visible, playlist-safe)
    and plays. P9.7 adds a hover prime cache: resolving during a ~200 ms
    dwell makes the subsequent Enter/click instant. Identity helpers live in
    onlinePick.ts (pure, unit-tested). */

export { pickFromRow, pickKey, type OnlinePick } from "@/lib/onlinePick";

/** Resolved candidates primed on hover (key → candidate). */
const specCache = new Map<string, Candidate>();
/** P9.9 single-flight: ONE shared promise per key across prime/play/add —
    a pick that lands while a prime is running awaits the same flight
    instead of spawning a duplicate yt-dlp chain (the old `Set` only
    deduped prime-vs-prime; add-vs-prime raced into two chains). Successes
    settle into specCache; rejections reach every waiter of that flight and
    the entry clears, so the next attempt starts fresh. */
const inflight = new Map<string, Promise<Candidate | null>>();

/** Resolve through the chain, reusing a primed or in-flight result. */
export async function resolveOnlinePick(pick: OnlinePick): Promise<Candidate | null> {
  const key = pickKey(pick);
  const primed = specCache.get(key);
  if (primed) {
    console.log(`[onlinePlay] prime hit ${key}`);
    return primed;
  }
  const flying = inflight.get(key);
  if (flying) {
    console.log(`[onlinePlay] awaiting in-flight ${key}`);
    return flying;
  }
  const flight = (async (): Promise<Candidate | null> => {
    const candidate = await resolveQuery({
      artist: pick.artist ?? "",
      title: pick.title,
      durationMs: pick.durationMs ?? null,
    });
    if (candidate) specCache.set(key, candidate);
    return candidate;
  })().finally(() => {
    inflight.delete(key);
  });
  inflight.set(key, flight);
  return flight;
}

/** P9.7 hover pre-resolve: fire-and-forget; a later pick reuses the spec.
    Failures are not cached (play/add show the toast). P9.9: starts a
    single-flight resolve — prime/play/add for the same key share it. */
export function primeOnlinePick(pick: OnlinePick): void {
  const key = pickKey(pick);
  if (specCache.has(key) || inflight.has(key)) return;
  resolveOnlinePick(pick).catch((err) =>
    console.warn("[onlinePlay] prime failed:", err),
  );
}

/** Persist the resolved candidate as a DB row and merge it into the
    in-memory library so the engine/player can resolve the id. */
export async function persistPick(pick: OnlinePick, candidate: Candidate): Promise<Track> {
  const row = await addStreamTrack({
    url: candidate.urlOrPath,
    artist: pick.artist ?? "",
    title: pick.title,
    durationMs: pick.durationMs ?? null,
    mbRecordingId: identityId(pick) || null,
  });
  const lib = useLibraryStore.getState();
  const existing = lib.tracks.some((t) => t.id === row.id);
  lib.setTracks(
    existing ? lib.tracks.map((t) => (t.id === row.id ? row : t)) : [...lib.tracks, row],
  );
  return row;
}

/** Returns true when playback started (callers close the overlay). */
export async function playOnlinePick(pick: OnlinePick): Promise<boolean> {
  try {
    const candidate = await resolveOnlinePick(pick);
    if (!candidate) {
      useUiStore.getState().pushToast({
        message: `No stream found for “${pick.title}”`,
        tone: "info",
      });
      return false;
    }
    const row = await persistPick(pick, candidate);
    usePlayerStore.getState().playQueue([row.id], 0);
    return true;
  } catch (err) {
    console.error("[search] resolve_query failed:", err);
    useUiStore.getState().pushToast({
      message: `Could not resolve “${pick.title}” — ${String(err)}`,
      tone: "info",
    });
    return false;
  }
}

/** P9.7 "+ Add to playlist" for a search hit: resolve + persist (reusing
    the prime cache), then link the DB row into the playlist. */
export async function addPickToPlaylist(
  pick: OnlinePick,
  playlistId: number,
): Promise<boolean> {
  try {
    const candidate = await resolveOnlinePick(pick);
    if (!candidate) {
      useUiStore.getState().pushToast({
        message: `No stream found for “${pick.title}” — not added`,
        tone: "info",
      });
      return false;
    }
    const row = await persistPick(pick, candidate);
    const ok = await addPlaylistTrack(playlistId, row.id);
    if (ok) {
      useUiStore.getState().pushToast({
        message: `Added “${pick.title}” to playlist`,
        tone: "info",
      });
    } else {
      // playlist_add_track only returns false for "already a member".
      useUiStore.getState().pushToast({
        message: "Already in this playlist",
        tone: "info",
      });
    }
    return ok;
  } catch (err) {
    console.error("[onlinePlay] add to playlist failed:", err);
    useUiStore.getState().pushToast({
      message: `Could not add “${pick.title}” — ${String(err)}`,
      tone: "info",
    });
    return false;
  }
}
