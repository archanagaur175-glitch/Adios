import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { inTauri } from "@/lib/api";
import { resolveOnlinePick, persistPick } from "@/lib/onlinePlay";
import type { OnlinePick } from "@/lib/onlinePick";
import { useDownloadStore } from "@/store/downloadStore";
import { useLibraryStore } from "@/store/libraryStore";
import { useUiStore } from "@/store/uiStore";

/** P12 pinned downloads: `download_track` streams a track into the disk
    cache and pins it (survives the size cap). Progress rides
    `download-progress` events (mirror of ytdlp-progress). Online search
    rows download via resolve → persist stream row → download. Store lives
    in store/downloadStore.ts (pure, unit-tested). */

interface DownloadDone {
  trackId: number;
  url: string;
  bytes: number;
  alreadyCached: boolean;
}

let listening = false;
/** One global listener (lazy — first download call wires it). */
function ensureListener(): void {
  if (!inTauri || listening) return;
  listening = true;
  void listen<{ trackId: number; received: number; total: number | null }>(
    "download-progress",
    (event) => {
      useDownloadStore.getState().setProgress(event.payload);
    },
  );
}

export async function downloadTrack(trackId: number): Promise<boolean> {
  if (!inTauri) return false;
  ensureListener();
  try {
    const done = await invoke<DownloadDone>("download_track", { trackId });
    useDownloadStore.getState().clearActive(trackId);
    useDownloadStore.getState().addCachedIds([trackId]);
    // The backend adopts the pinned URL as tracks.url — mirror it in the
    // in-memory library so playback + badges use the same key.
    const lib = useLibraryStore.getState();
    lib.setTracks(lib.tracks.map((t) => (t.id === trackId ? { ...t, url: done.url } : t)));
    return true;
  } catch (err) {
    useDownloadStore.getState().clearActive(trackId);
    console.error("[download] failed:", err);
    useUiStore.getState().pushToast({ message: String(err), tone: "danger" });
    return false;
  }
}

/** Sequential playlist download — one toast summary at the end. */
export async function downloadTracks(trackIds: number[]): Promise<number> {
  let done = 0;
  for (const id of trackIds) {
    if (await downloadTrack(id)) done += 1;
  }
  return done;
}

/** Resolve an online search hit → persist the stream row → download. */
export async function downloadOnlinePick(pick: OnlinePick): Promise<boolean> {
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
    return downloadTrack(row.id);
  } catch (err) {
    console.error("[download] resolve failed:", err);
    useUiStore.getState().pushToast({ message: String(err), tone: "danger" });
    return false;
  }
}

/** Batch "already on disk" check for Downloaded ✓ badges. */
export async function refreshCachedIds(trackIds: number[]): Promise<void> {
  if (!inTauri || trackIds.length === 0) return;
  const ids = await invoke<number[]>("cached_track_ids", { trackIds });
  useDownloadStore.getState().setCachedIds(ids);
}
