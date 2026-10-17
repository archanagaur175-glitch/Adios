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
}

/** Relevance-ordered iTunes rows (cache-first, ~400 ms spacing Rust-side). */
export async function searchItunes(query: string): Promise<ItunesTrack[]> {
  if (!inTauri || !query.trim()) return [];
  return invoke<ItunesTrack[]>("search_itunes", { query });
}

/** Persist an online pick as a real 'stream' library row. Dedupe by
    `mbRecordingId` (re-pick refreshes the URL); returns the full row. */
export async function addStreamTrack(input: {
  url: string;
  artist: string;
  title: string;
  durationMs?: number | null;
  mbRecordingId?: string | null;
}): Promise<Track> {
  return invoke<Track>("add_stream_track", {
    url: input.url,
    artist: input.artist,
    title: input.title,
    durationMs: input.durationMs ?? null,
    mbRecordingId: input.mbRecordingId ?? null,
  });
}
