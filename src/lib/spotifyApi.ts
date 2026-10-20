import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "@/lib/api";
import type { RecCandidate } from "@/lib/recommend";

/** P17 (D18): anonymous Spotify playlist import — the Rust client fetches
    the public embed SSR + token-paginated API; this is the thin wrapper. */

export interface SpotifyTrack {
  uri: string;
  title: string;
  artist: string;
  durationMs: number | null;
  /** P21: album art — track-page visualIdentity or the oEmbed fill pass
      (Rust sends null when art could not be fetched; import still runs). */
  artworkUrl?: string | null;
}

export interface SpotifyPlaylist {
  name: string;
  total: number;
  tracks: SpotifyTrack[];
}

export async function fetchSpotifyPlaylist(url: string): Promise<SpotifyPlaylist> {
  if (!inTauri) throw new Error("Spotify import runs inside the Adios app");
  return invoke<SpotifyPlaylist>("spotify_fetch_playlist", { url });
}

/** Spotify rows → save-engine candidates: identity is the uri via `mbid`
    (stored as `spotify:track:<id>` — never an iTunes id, D18). Rows whose
    metadata fetch failed (empty title) are dropped — they would fail the
    resolver anyway and the engine counts honest skips either way. */
export function spotifyCandidates(pl: SpotifyPlaylist): RecCandidate[] {
  return pl.tracks
    .filter((t) => t.title.trim() !== "")
    .map((t) => ({
      mbid: t.uri,
      title: t.title,
      artist: t.artist,
      durationMs: t.durationMs,
      artworkUrl: t.artworkUrl,
    }));
}
