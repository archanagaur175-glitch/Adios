import { useEffect } from "react";
import { searchLocal, searchMetadata } from "@/lib/api";
import { fetchOnlineTracks } from "./onlineTracks";
import { useSearchStore } from "@/store/searchStore";
import type { MbArtistItem, MbReleaseItem } from "@/lib/types";

/** Search data layers for the Ctrl+K overlay (P5.3/P9.5/P9.7 + P9.8):
    local FTS5 on every keystroke; online after a 275 ms debounce (D12).
    P9.8: the three online layers are DECOUPLED — tracks (iTunes→MB via
    fetchOnlineTracks), MB albums and MB artists each land independently;
    one layer failing or stalling never hides another's results (bug #3:
    Promise.all head-of-line blocking + all-or-nothing rejection showed
    "No matches" while good rows sat in memory). Stale runs are dropped. */

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

  // Online layer: one debounce, then three independent layers. pending only
  // clears when ALL three have settled (or the query changed).
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
      let open = 3;
      const done = () => {
        open -= 1;
        if (open === 0 && !cancelled) {
          useSearchStore.getState().setOnlinePending(false);
        }
      };

      fetchOnlineTracks(q)
        .then((tracks) => {
          if (!cancelled) useSearchStore.getState().setOnlineResults({ tracks });
        })
        .catch((err) => console.error("[search] tracks layer failed:", err))
        .finally(done);

      searchMetadata<MbReleaseItem>("release", q)
        .then((rel) => {
          if (cancelled) return;
          useSearchStore.getState().setOnlineResults({
            albums: rel.items.map((r) => ({
              mbid: r.id,
              title: r.title,
              artist: creditNames(r.artistCredit),
            })),
          });
        })
        .catch((err) => console.error("[search] albums layer failed:", err))
        .finally(done);

      searchMetadata<MbArtistItem>("artist", q)
        .then((art) => {
          if (cancelled) return;
          useSearchStore.getState().setOnlineResults({
            artists: art.items.map((r) => ({
              mbid: r.id,
              title: r.name,
              artist: r.disambiguation ?? "",
            })),
          });
        })
        .catch((err) => console.error("[search] artists layer failed:", err))
        .finally(done);
    }, DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [query]);
}
