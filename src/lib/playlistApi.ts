import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "@/lib/api";
import type {
  ImportResult,
  PlaylistDetail,
  PlaylistSummary,
} from "@/lib/types";

/** Playlist commands (P8.2) — thin invoke wrappers; file IO happens in Rust. */

export async function listPlaylists(): Promise<PlaylistSummary[]> {
  if (!inTauri) return [];
  return invoke<PlaylistSummary[]>("playlist_list");
}

export async function getPlaylist(id: number): Promise<PlaylistDetail | null> {
  if (!inTauri) return null;
  return invoke<PlaylistDetail | null>("playlist_detail", { id });
}

/** New playlist id, or -1 outside Tauri. */
export async function createPlaylist(name: string): Promise<number> {
  if (!inTauri) return -1;
  return invoke<number>("playlist_create", { name });
}

export async function renamePlaylist(id: number, name: string): Promise<boolean> {
  if (!inTauri) return false;
  return invoke<boolean>("playlist_rename", { id, name });
}

/** False when the playlist does not exist. */
export async function deletePlaylist(id: number): Promise<boolean> {
  if (!inTauri) return false;
  return invoke<boolean>("playlist_delete", { id });
}

/** False when the playlist is missing or already contains the track. */
export async function addPlaylistTrack(playlistId: number, trackId: number): Promise<boolean> {
  if (!inTauri) return false;
  return invoke<boolean>("playlist_add_track", { playlistId, trackId });
}

export async function removePlaylistTrack(playlistId: number, trackId: number): Promise<boolean> {
  if (!inTauri) return false;
  return invoke<boolean>("playlist_remove_track", { playlistId, trackId });
}

/** Full new order; false when the ids are not exactly the current members. */
export async function reorderPlaylist(playlistId: number, trackIds: number[]): Promise<boolean> {
  if (!inTauri) return false;
  return invoke<boolean>("playlist_reorder", { playlistId, trackIds });
}

/** Writes pretty JSON to `path`; returns the exported track count. */
export async function exportPlaylistFile(id: number, path: string): Promise<number> {
  if (!inTauri) return -1;
  return invoke<number>("playlist_export_file", { id, path });
}

/** Reads `path` (version-guarded) and creates the playlist. */
export async function importPlaylistFile(path: string): Promise<ImportResult | null> {
  if (!inTauri) return null;
  return invoke<ImportResult>("playlist_import_file", { path });
}
