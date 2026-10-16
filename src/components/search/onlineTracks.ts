import { searchMetadata } from "@/lib/api";
import { searchItunes } from "@/lib/onlineApi";
import { rankMbTracks } from "./rankMb";
import type { MbRecordingItem } from "@/lib/types";

/** P9.8: the online TRACK layer, shared by the Ctrl+K overlay and the
    playlist AddTracks picker. iTunes-first (keyless, relevance order);
    MusicBrainz recordings (P9.6 ripoff-aware ranker) only when iTunes is
    empty or failed. Errors are layer-local: an iTunes failure falls through
    to MB, and neither discards the other's results — this is what fixed
    the "No matches" flash (P9.8 bug #3: Promise.all used to throw iTunes
    rows away whenever an MB albums/artists call failed). */

export interface OnlineTrackHit {
  /** "itunes:<id>" or an MB recording UUID — dedupe key in the DB. */
  mbid: string;
  itunesId?: number;
  title: string;
  artist: string;
  album?: string;
  durationMs?: number;
}

const creditNames = (
  credit?: { artist?: { name: string } | null }[] | null,
): string =>
  (credit ?? [])
    .map((c) => c.artist?.name)
    .filter((n): n is string => Boolean(n))
    .join(", ");

async function mbRecordingTracks(q: string): Promise<OnlineTrackHit[]> {
  const rec = await searchMetadata<MbRecordingItem>("recording", q);
  return rankMbTracks(
    q,
    rec.items.map((r) => ({
      mbid: r.id,
      title: r.title,
      artist: creditNames(r.artistCredit),
      durationMs: r.length ?? undefined,
      score: r.score,
    })),
  ).map((r) => ({
    mbid: r.mbid,
    title: r.title,
    artist: r.artist,
    durationMs: r.durationMs,
  }));
}

/** Relevance-ordered track hits for `q`. Throws only when BOTH sources
    failed (callers log + treat as an empty layer). */
export async function fetchOnlineTracks(q: string): Promise<OnlineTrackHit[]> {
  let itunesRows: OnlineTrackHit[] | null = null;
  try {
    const rows = await searchItunes(q);
    itunesRows = rows.map((d) => ({
      mbid: `itunes:${d.itunesId}`,
      itunesId: d.itunesId,
      title: d.title,
      artist: d.artist,
      album: d.album ?? undefined,
      durationMs: d.durationMs ?? undefined,
    }));
    if (itunesRows.length > 0) return itunesRows;
  } catch (err) {
    console.error("[search] itunes failed:", err);
  }
  try {
    const mb = await mbRecordingTracks(q);
    if (mb.length > 0) return mb;
  } catch (err) {
    if (itunesRows !== null) return []; // iTunes answered (empty) — honest miss
    throw err; // both failed — let the caller log one clear error
  }
  return [];
}
