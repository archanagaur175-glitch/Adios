import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "@/lib/api";
import type { Track } from "@/lib/types";

/** Play history (P10): `record_play` bumps tracks.play_count/last_played_at;
    `list_play_history` returns most-recent-first (LIMIT 200 in Rust). */

export async function recordPlay(id: number): Promise<void> {
  if (!inTauri) return;
  await invoke("record_play", { id });
}

export async function listPlayHistory(): Promise<Track[]> {
  if (!inTauri) return [];
  return invoke<Track[]>("list_play_history");
}
