import { persistPick, resolveOnlinePick } from "@/lib/onlinePlay";
import { resolveManyPicks } from "@/lib/batchResolve";
import { findLibraryMatch, type RecCandidate } from "@/lib/recommend";
import { toPick } from "@/lib/recQueue";
import { addPlaylistTrack, createPlaylist } from "@/lib/playlistApi";
import { useLibraryStore } from "@/store/libraryStore";

/** P16/P17/P20: "save picks to a playlist" engine — `playlist_create`, then
    one resolve → persist → `playlist_add_track` per pick. P20 (D34): before
    the loop, `resolveManyPicks` prefetches every pick that is NOT already in
    the library as ≤3 concurrent 8-query yt-dlp chunks (reservations land in
    the prime/inflight maps, so the loop's `resolveOnlinePick` calls are
    cache hits); any latch/failure falls back to today's per-pick chain —
    Android and yt-dlp-less desktops unchanged. DB writes stay sequential,
    `onProgress` fires after every item (`Saving 4/12…`, monotonic).
    Single-flight: one save at a time, a second call while running returns
    null. Failures are SKIPPED, never thrown. Shared by the mood view
    (P16.6) and the Spotify import (P17.3). */

export interface SaveResult {
  playlistId: number;
  name: string;
  added: number;
  skipped: number;
}

let running = false;

export function isSaving(): boolean {
  return running;
}

export async function savePicksToPlaylist(
  name: string,
  picks: RecCandidate[],
  onProgress: (done: number, total: number) => void,
): Promise<SaveResult | null> {
  if (running || picks.length === 0) return null;
  running = true;
  try {
    const playlistId = await createPlaylist(name);
    if (playlistId < 0) return null;
    // P20.2: prefetch resolves for the picks the loop will actually resolve
    // (library matches never reach it — fast-path rows below stay as-is).
    resolveManyPicks(
      picks
        .filter((p) => findLibraryMatch(p, useLibraryStore.getState().tracks) == null)
        .map(toPick),
    );
    let added = 0;
    let skipped = 0;
    for (let i = 0; i < picks.length; i++) {
      const pick = picks[i];
      // Already in the library → its row; else resolve + persist (dedupe by
      // itunes:<id> identity inside add_stream_track re-picks).
      let rowId = findLibraryMatch(pick, useLibraryStore.getState().tracks);
      if (rowId == null) {
        try {
          const candidate = await resolveOnlinePick(toPick(pick));
          if (candidate) {
            const row = await persistPick(toPick(pick), candidate);
            rowId = row.id;
          }
        } catch (err) {
          console.warn("[playlistSave] resolve failed:", err);
        }
      }
      if (rowId == null) {
        skipped++;
      } else {
        const ok = await addPlaylistTrack(playlistId, rowId);
        if (ok) added++;
        else skipped++;
      }
      onProgress(i + 1, picks.length);
    }
    return { playlistId, name, added, skipped };
  } finally {
    running = false;
  }
}
