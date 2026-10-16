import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "@/lib/api";
import type { Track } from "@/lib/types";

/** Session persistence (P8.3): queue lives in the SQLite `queue` table;
    position/volume/muted ride the generic settings keys. */

export interface QueueSnapshot {
  trackIds: number[];
  queueIndex: number;
  /** Rows lost since the last save (deleted tracks). */
  dropped: number;
}

export async function saveQueue(trackIds: number[], queueIndex: number): Promise<void> {
  if (!inTauri) return;
  await invoke("session_save_queue", { trackIds, queueIndex });
}

export async function loadQueue(): Promise<QueueSnapshot | null> {
  if (!inTauri) return null;
  return invoke<QueueSnapshot>("session_load_queue");
}

/** Remote tracks with a disk-cache entry (the "Cached" offline view). */
export async function listCachedTracks(): Promise<Track[]> {
  if (!inTauri) return [];
  return invoke<Track[]>("list_cached_tracks");
}
