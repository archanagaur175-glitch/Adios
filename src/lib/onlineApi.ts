import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "@/lib/api";
import type { Track } from "@/lib/types";

/** Online playback APIs (P9.7): iTunes keyless relevance-ordered track
    search (planned as Deezer; their anonymous API now returns empty data —
    see meta/itunes.rs) and persistent 'stream' library rows. Split from
    api.ts for the 200-line budget. */

export interface ItunesTrack {
  itunesId: number;
  title: string;
  artist: string;
  album?: string | null;
  durationMs?: number | null;
  /** 100×100 artwork URL from the iTunes response (P12 covers). */
  artworkUrl?: string | null;
  /** P16: Apple's primaryGenreName ("Pop", "Electronic", …). */
  genre?: string | null;
  /** P16: ISO-8601 release stamp from the search response. */
  releaseDate?: string | null;
}

/** Relevance-ordered iTunes rows (cache-first, ~400 ms spacing Rust-side). */
export async function searchItunes(query: string): Promise<ItunesTrack[]> {
  if (!inTauri || !query.trim()) return [];
  return invoke<ItunesTrack[]>("search_itunes", { query });
}

/** P16.5: one row of Apple's RSS top-songs chart (no duration — RSS has
    none; structurally a `RecCandidate`, so it feeds picks directly). */
export interface RssSong {
  itunesId: number;
  title: string;
  artist: string;
  artworkUrl?: string | null;
  genre?: string | null;
  releaseDate?: string | null;
}

/** RSS chart for Home's Discover shelf — 6 h age-aware cache Rust-side;
    a failed refresh serves the last chart, a cold failure throws (the
    shelf falls back to a seeded search and hides itself if that fails too). */
export async function rssTopSongs(): Promise<RssSong[]> {
  if (!inTauri) return [];
  return invoke<RssSong[]>("rss_top_songs");
}

/** Persist an online pick as a real 'stream' library row. Dedupe by
    `mbRecordingId` (re-pick refreshes the URL); returns the full row.
    P13: `artworkUrl` (iTunes) is downloaded and cached as the cover. */
export async function addStreamTrack(input: {
  url: string;
  artist: string;
  title: string;
  durationMs?: number | null;
  mbRecordingId?: string | null;
  artworkUrl?: string | null;
  /** P16: iTunes primaryGenreName persisted on the stream row. */
  genre?: string | null;
}): Promise<Track> {
  return invoke<Track>("add_stream_track", {
    url: input.url,
    artist: input.artist,
    title: input.title,
    durationMs: input.durationMs ?? null,
    mbRecordingId: input.mbRecordingId ?? null,
    artworkUrl: input.artworkUrl ?? null,
    genre: input.genre ?? null,
  });
}
