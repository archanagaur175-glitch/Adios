import { invoke } from "@tauri-apps/api/core";
import type { Track } from "@/lib/types";

/** True when running inside the Tauri webview (not plain vite/preview). */
export const inTauri = "__TAURI_INTERNALS__" in window;

/** All settings: key → parsed value. Empty map outside Tauri or on error. */
export async function getSettings(): Promise<Record<string, unknown>> {
  if (!inTauri) return {};
  try {
    const raw = await invoke<Record<string, string>>("get_settings");
    const parsed: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(raw)) {
      try {
        parsed[key] = JSON.parse(value);
      } catch {
        // skip malformed rows rather than breaking startup
      }
    }
    return parsed;
  } catch (error) {
    console.error("[api] get_settings failed:", error);
    return {};
  }
}

/** Persist one setting (value is JSON-serialized on this side). */
export async function setSetting(key: string, value: unknown): Promise<void> {
  if (!inTauri) return;
  try {
    await invoke("set_setting", { key, valueJson: JSON.stringify(value) });
  } catch (error) {
    console.error(`[api] set_setting(${key}) failed:`, error);
  }
}

// ---------- Library ----------

/** Fields accepted when importing a track (camelCase → Rust NewTrack). */
export interface NewTrackInput {
  source: string;
  path?: string | null;
  url?: string | null;
  artist: string;
  title: string;
  album?: string | null;
  durationMs?: number | null;
  year?: number | null;
  trackNo?: number | null;
  discNo?: number | null;
  fileSize?: number | null;
  mtime?: number | null;
}

export async function listTracks(): Promise<Track[]> {
  if (!inTauri) return [];
  return invoke<Track[]>("list_tracks");
}

export async function getTrack(id: number): Promise<Track | null> {
  if (!inTauri) return null;
  return invoke<Track | null>("get_track", { id });
}

/** Returns the track id; re-inserting an existing path returns its current id. */
export async function insertTrack(track: NewTrackInput): Promise<number> {
  if (!inTauri) return -1;
  return invoke<number>("insert_track", { track });
}

/** Returns the number of rows deleted (0 or 1). */
export async function deleteTrack(id: number): Promise<number> {
  if (!inTauri) return 0;
  return invoke<number>("delete_track", { id });
}
