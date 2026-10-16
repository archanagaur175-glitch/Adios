import { useEffect } from "react";
import { searchLocal, searchMetadata } from "@/lib/api";
import { rankMbTracks } from "./rankMb";
import { useSearchStore } from "@/store/searchStore";
import type { MbArtistItem, MbRecordingItem, MbReleaseItem } from "@/lib/types";

/** Search data layers for the Ctrl+K overlay (extracted from SearchOverlay
    in P9.5 for the 200-line budget): local FTS5 on every keystroke, online
    MusicBrainz after a 275 ms debounce (D12). Both write the search store;
    stale responses are dropped. */

const DEBOUNCE_MS = 275;

const creditNames = (
  credit?: { artist?: { name: string } | null }[] | null,
): string =>
  (credit ?? [])
    .map((c) => c.artist?.name)
    .filter((n): n is string => Boolean(n))
    .join(", ");

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

  // Online layer: 275 ms debounce, three MB kinds in parallel (the client
  // serializes them through the 1-req/s queue), stale runs cancelled.
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
        searchMetadata<MbRecordingItem>("recording", q),
        searchMetadata<MbReleaseItem>("release", q),
        searchMetadata<MbArtistItem>("artist", q),
      ])
        .then(([rec, rel, art]) => {
          if (cancelled) return;
          // P9.6 fix #2: re-rank tracks so real recordings beat karaoke/
          // tribute/lullaby ripoffs (MB has no popularity field).
          const tracks = rankMbTracks(
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
        .catch((err) => console.error("[search] musicbrainz failed:", err))
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
