import { useEffect } from "react";
import { searchLocal, searchMetadata } from "@/lib/api";
import { searchItunes } from "@/lib/onlineApi";
import { rankMbTracks } from "./rankMb";
import { useSearchStore } from "@/store/searchStore";
import type { MbArtistItem, MbRecordingItem, MbReleaseItem } from "@/lib/types";

/** Search data layers for the Ctrl+K overlay (P5.3/P9.5 + P9.7): local
    FTS5 on every keystroke, online after a 275 ms debounce (D12). Online
    tracks are iTunes-first (single keyless request, relevance order);
    MusicBrainz supplies albums/artists always and recordings only when
    iTunes returns nothing. Stale responses are dropped. */

const DEBOUNCE_MS = 275;

const creditNames = (
  credit?: { artist?: { name: string } | null }[] | null,
): string =>
  (credit ?? [])
    .map((c) => c.artist?.name)
    .filter((n): n is string => Boolean(n))
    .join(", ");

/** MB recording rows through the P9.6 ripoff-aware ranker (fallback path). */
async function mbTrackFallback(q: string): Promise<
  { mbid: string; title: string; artist: string; durationMs?: number }[]
> {
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

export function useSearchLayers(query: string): void {
  // Local layer: query on every keystroke, stale responses dropped.
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      useSearchStore.getState().setLocalResults([]);
      return;
    }
    let cancelled = false;
    searchLocal(q)
      .then((rows) => {
        if (!cancelled) useSearchStore.getState().setLocalResults(rows);
      })
      .catch((err) => console.error("[search] local failed:", err));
    return () => {
      cancelled = true;
    };
  }, [query]);

  // Online layer: 275 ms debounce; iTunes + MB albums/artists in parallel
  // (separate Rust queues); MB recordings only if iTunes came back empty.
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      const s = useSearchStore.getState();
      s.setOnlineResults({ tracks: [], albums: [], artists: [] });
      s.setOnlinePending(false);
      return;
    }
    let cancelled = false;
    useSearchStore.getState().setOnlinePending(true);
    const timer = window.setTimeout(() => {
      Promise.all([
        searchItunes(q),
        searchMetadata<MbReleaseItem>("release", q),
        searchMetadata<MbArtistItem>("artist", q),
      ])
        .then(async ([itunes, rel, art]) => {
          if (cancelled) return;
          const tracks =
            itunes.length > 0
              ? itunes.map((d) => ({
                  mbid: `itunes:${d.itunesId}`,
                  title: d.title,
                  artist: d.artist,
                  album: d.album ?? undefined,
                  durationMs: d.durationMs ?? undefined,
                  itunesId: d.itunesId,
                }))
              : await mbTrackFallback(q);
          if (cancelled) return;
          useSearchStore.getState().setOnlineResults({
            tracks,
            albums: rel.items.map((r) => ({
              mbid: r.id,
              title: r.title,
              artist: creditNames(r.artistCredit),
            })),
            artists: art.items.map((r) => ({
              mbid: r.id,
              title: r.name,
              artist: r.disambiguation ?? "",
            })),
          });
        })
        .catch((err) => console.error("[search] online failed:", err))
        .finally(() => {
          if (!cancelled) useSearchStore.getState().setOnlinePending(false);
        });
    }, DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [query]);
}
