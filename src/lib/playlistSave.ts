import { persistPick, resolveOnlinePick } from "@/lib/onlinePlay";
import { findLibraryMatch, type RecCandidate } from "@/lib/recommend";
import { toPick } from "@/lib/recQueue";
import { addPlaylistTrack, createPlaylist } from "@/lib/playlistApi";
import { useLibraryStore } from "@/store/libraryStore";

/** P16/P17: sequential "save picks to a playlist" engine —
    `playlist_create`, then one resolve → persist → `playlist_add_track`
    per pick (slow but honest: no parallel resolver chains, library rows
    reused via `findLibraryMatch` fast-path). Shared by the mood view
    (P16.6) and the Spotify import (P17.3). Single-flight: one save at a
    time, a second call while running returns null. Failures are SKIPPED,
    never thrown; `onProgress` fires after every item for the inline
    `Saving 4/12…` readout. */

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
