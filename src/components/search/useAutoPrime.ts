import { useEffect } from "react";
import { primeOnlinePick } from "@/lib/onlinePlay";
import type { OnlineResult } from "@/store/searchStore";

/** P9.8 #4: auto-prime the top track rows the moment THIS query's results
    land — the user's reading time absorbs the 2–4 s yt-dlp spawn, so
    picking a result that sat on screen ≥ ~2 s resolves near-instantly.
    primeOnlinePick itself dedupes (spec cache + in-flight set), so effect
    re-runs are free. Short queries are skipped — a 2-char blast isn't a
    deliberate pick. The 200 ms dwell prime (keyboard nav) lives in
    SearchOverlay and stays complementary. */

const AUTO_PRIME_COUNT = 2;
const AUTO_PRIME_MIN_QUERY = 3;

export function useAutoPrime(open: boolean, query: string, tracks: OnlineResult[]): void {
  useEffect(() => {
    if (!open || query.trim().length < AUTO_PRIME_MIN_QUERY) return;
    for (const t of tracks.slice(0, AUTO_PRIME_COUNT)) {
      if (!t.mbid) continue;
      primeOnlinePick({
        mbid: t.mbid,
        itunesId: t.itunesId,
        title: t.title,
        artist: t.artist,
        durationMs: t.durationMs,
      });
    }
  }, [open, query, tracks]);
}
