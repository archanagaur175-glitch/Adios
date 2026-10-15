/** Core data models — mirror src-tauri schema (docs/ARCHITECTURE.md §3.1). */

export type AudioSource = "local" | "stream";

export interface Track {
  id: number;
  /** Absolute path for local files; null for streams. */
  path?: string | null;
  source: AudioSource;
  /** Stream URL or resolver ref for non-local tracks. */
  url?: string | null;
  artist: string;
  title: string;
  album?: string | null;
  durationMs?: number | null;
  year?: number | null;
  trackNo?: number | null;
  discNo?: number | null;
  coverPath?: string | null;
  /** Dynamic accent colors extracted from cover art (OKLCH-able hex). */
  accent?: string | null;
  accent2?: string | null;
  /** [r, g, b] triplets cached from the 32×32 cover downscale. */
  dominantColors?: [string, string, string] | null;
  addedAt: string;
  playCount: number;
}

export interface Playlist {
  id: number;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export type QualityTier = "low" | "med" | "high";
