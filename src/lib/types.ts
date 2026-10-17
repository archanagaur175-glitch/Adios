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
  /** Dynamic accent colors extracted from cover art (hex). */
  accent?: string | null;
  accent2?: string | null;
  /** JSON array of 3 dominant colors from the cover downscale, as stored by Rust. */
  dominantColorsJson?: string | null;
  /** MusicBrainz identifiers (written by import enrichment, removed in P12 —
      historical rows may still carry them). */
  mbRecordingId?: string | null;
  mbReleaseId?: string | null;
  /** File size in bytes (local files). */
  fileSize?: number | null;
  mtime?: number | null;
  addedAt: string;
  playCount: number;
  /** UTC stamp written on every play (null until first play). */
  lastPlayedAt?: string | null;
}

export interface Playlist {
  id: number;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export type QualityTier = "low" | "med" | "high";

// ---------- Search (P5.1/P5.3) ----------

/** One page of MusicBrainz search results (mirrors Rust `SearchResults`). */
export interface SearchPage<T> {
  count: number;
  offset: number;
  items: T[];
}

/** Minimal MB shapes the search UI consumes (see mb_types.rs). */
export interface MbArtistItem {
  id: string;
  score?: number | null;
  name: string;
  disambiguation?: string | null;
}

export interface MbCreditRef {
  artist?: { name: string } | null;
}

export interface MbRecordingItem {
  id: string;
  score?: number | null;
  title: string;
  length?: number | null;
  artistCredit?: MbCreditRef[] | null;
}

export interface MbReleaseItem {
  id: string;
  score?: number | null;
  title: string;
  artistCredit?: MbCreditRef[] | null;
  date?: string | null;
}

// ---------- Disk cache (P8.1) ----------

/** Result of one cache eviction pass (mirrors Rust `EvictStats`). */
export interface EvictStats {
  /** Rows deleted: dangling (file gone) + beyond-cap (LRU order). */
  removed: number;
  /** Stale `.part-*` temp files swept. */
  removedParts: number;
  /** Unreferenced cache files swept (crash leftovers). */
  removedOrphans: number;
  freedBytes: number;
  remainingBytes: number;
}

// ---------- Playlists (P8.2) ----------

/** List row: playlist metadata + member count. */
export interface PlaylistSummary {
  id: number;
  name: string;
  createdAt: string;
  updatedAt: string;
  trackCount: number;
}

/** Detail payload: metadata + members in position order. */
export interface PlaylistDetail {
  id: number;
  name: string;
  createdAt: string;
  updatedAt: string;
  tracks: Track[];
}

/** Stable identity for one exported track (never a row id). */
export interface TrackRef {
  path?: string | null;
  url?: string | null;
  mbRecordingId?: string | null;
  artist: string;
  title: string;
  durationMs?: number | null;
}

/** Versioned JSON export shape (`version` guards schema drift). */
export interface PlaylistExport {
  version: number;
  name: string;
  tracks: TrackRef[];
}

/** Result of one import: how many refs resolved vs. were missing. */
export interface ImportResult {
  playlistId: number;
  added: number;
  missing: number;
}

// ---------- Lyrics (P7.1) ----------

/** LRCLIB answer for one track (mirrors Rust `LyricsPayload`). */
export interface LyricsPayload {
  found: boolean;
  /** Raw LRC text (timestamps `[mm:ss.xx]`); null when plain-only. */
  synced?: string | null;
  plain?: string | null;
  instrumental: boolean;
  lrclibId?: number | null;
  /** "cache" | "network" | "none". */
  source: string;
}
