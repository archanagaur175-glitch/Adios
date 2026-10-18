import { invoke } from "@tauri-apps/api/core";
import type { EvictStats, LyricsPayload, SearchPage, Track } from "@/lib/types";

/** True when running inside the Tauri webview (not plain vite/preview). */
export const inTauri = "__TAURI_INTERNALS__" in window;

/** True on the Android WebView — desktop-only features (yt-dlp updater,
    native dialogs) are hidden there; online play is Piped-only (D19). */
export const isAndroid = (): boolean =>
  typeof navigator !== "undefined" && navigator.userAgent.includes("Android");

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

// ---------- Loopback server ----------

/** Connection info for the Rust loopback audio server (Invariant 1). */
export interface ServerInfo {
  port: number;
  token: string;
  baseUrl: string;
}

let cachedServerInfo: ServerInfo | null = null;

/** Fetched once per session via `server_info`; null outside Tauri or on error. */
export async function getServerInfo(): Promise<ServerInfo | null> {
  if (!inTauri) return null;
  if (cachedServerInfo) return cachedServerInfo;
  try {
    cachedServerInfo = await invoke<ServerInfo>("server_info");
    return cachedServerInfo;
  } catch (error) {
    console.error("[api] server_info failed:", error);
    return null;
  }
}

/** Loopback URL for a local file path (always goes through the tokened server). */
export async function buildMediaUrl(path: string): Promise<string | null> {
  const info = await getServerInfo();
  if (!info) return null;
  return `${info.baseUrl}/media/${info.token}?path=${encodeURIComponent(path)}`;
}

/** Loopback URL for a remote stream URL (proxied + disk-cached, P4.3/P4.4). */
export async function buildRemoteMediaUrl(url: string): Promise<string | null> {
  const info = await getServerInfo();
  if (!info) return null;
  return `${info.baseUrl}/media/${info.token}?url=${encodeURIComponent(url)}`;
}

/** Media URL for a track — local path or remote URL, both via the loopback
    server (Invariant 1: ALL audio goes through the proxy). */
export async function trackMediaUrl(
  track: Pick<Track, "path" | "url">,
): Promise<string | null> {
  if (track.path) return buildMediaUrl(track.path);
  if (track.url) {
    return track.url.startsWith("http")
      ? buildRemoteMediaUrl(track.url)
      : buildMediaUrl(track.url);
  }
  return null;
}

// ---------- Scan ----------

/** Summary returned by one folder scan (mirrors Rust ScanSummary). */
export interface ScanSummary {
  folder: string;
  added: number;
  skipped: number;
  failed: number;
  errors: string[];
  elapsedMs: number;
}

/** Scan a folder for audio files and import them. Returns null outside Tauri. */
export async function scanFolder(folder: string): Promise<ScanSummary | null> {
  if (!inTauri) return null;
  return invoke<ScanSummary>("scan_folder", { folder });
}

/** P18.1c: directory listing for the in-app folder picker. */
export interface DirListing {
  parent: string | null;
  dirs: string[];
}

/** List sub-directories of a path (sorted, hidden `.` entries excluded). */
export async function listDirectory(path: string): Promise<DirListing> {
  if (!inTauri) throw new Error("folder listing needs the app");
  return invoke<DirListing>("list_directory", { path });
}

// ---------- Search (P5.3) ----------

/** Instant local FTS5 search (≤ 50 rows); empty input short-circuits. */
export async function searchLocal(query: string): Promise<Track[]> {
  if (!inTauri || !query.trim()) return [];
  return invoke<Track[]>("search_local", { query });
}

/** MusicBrainz search via the cache-first MB client (queue + meta_cache). */
export async function searchMetadata<T>(
  kind: "artist" | "recording" | "release",
  query: string,
): Promise<SearchPage<T>> {
  if (!inTauri || !query.trim()) return { count: 0, offset: 0, items: [] };
  return invoke<SearchPage<T>>("search_metadata", { kind, query });
}

// ---------- Lyrics (P7.1) ----------

export async function getLyrics(trackId: number): Promise<LyricsPayload> {
  if (!inTauri) return { found: false, instrumental: false, source: "none" };
  return invoke<LyricsPayload>("get_lyrics", { trackId });
}

// ---------- Disk cache (P8.1) ----------

/** Apply the `cacheSizeMb` cap now; zeros outside Tauri. */
export async function runCacheEviction(): Promise<EvictStats> {
  if (!inTauri)
    return { removed: 0, removedParts: 0, removedOrphans: 0, freedBytes: 0, remainingBytes: 0 };
  return invoke<EvictStats>("run_cache_eviction");
}
