import { useEffect, useState } from "react";
import { inTauri } from "@/lib/api";
import { rssTopSongs, searchItunes } from "@/lib/onlineApi";
import { playOnlinePick, primeOnlinePick } from "@/lib/onlinePlay";
import {
  findLibraryMatch,
  planPicks,
  type RecCandidate,
} from "@/lib/recommend";
import { toPick } from "@/lib/recQueue";
import { seedQueries } from "@/lib/taste";
import { useLibraryStore } from "@/store/libraryStore";
import { CoverArt } from "@/components/common/CoverArt";

/** P16 section D: "Discover new" — Apple's RSS top-songs chart (Rust keeps
    a 6 h age-aware cache and serves the last chart when the refresh fails),
    minus everything already in the library. Cold feed failure falls back to
    a seeded iTunes search; if that fails too the section hides silently
    (Home must never hard-fail on the network). Cards stream on click and
    hover-prime through the P9.7 machinery. */

const CARD_W = 144;
const SHOW_COUNT = 12;

export function DiscoverNew() {
  const tracks = useLibraryStore((s) => s.tracks);
  const [picks, setPicks] = useState<RecCandidate[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!inTauri) return;
    let cancelled = false;
    setLoading(true);
    void (async () => {
      let cands: RecCandidate[];
      try {
        cands = await rssTopSongs();
      } catch {
        const groups: RecCandidate[][] = [];
        for (const seed of seedQueries(tracks, Date.now(), 2)) {
          try {
            groups.push(await searchItunes(seed));
          } catch {
            // one dead seed → fewer groups
          }
        }
        cands = planPicks(groups, new Set(), SHOW_COUNT);
      }
      if (cancelled) return;
      // Library owns its rows (identity `itunes:<id>` AND normalized meta).
      const fresh = cands.filter((c) => findLibraryMatch(c, tracks) == null);
      setPicks(fresh.slice(0, SHOW_COUNT));
      setLoading(false);
    })().catch(() => {
      if (cancelled) return;
      setPicks([]);
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [tracks]);

  if (!loading && picks.length === 0) return null;

  return (
    <section>
      <header className="mb-3 flex items-end justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-18 font-display">Discover new</h2>
          <p className="text-12 text-ink-3">Apple&apos;s top songs right now</p>
        </div>
      </header>

      {loading && picks.length === 0 ? (
        <div className="flex gap-3" aria-hidden="true">
          {[0, 1, 2, 3, 4].map((i) => (
            <div
              key={i}
              className="shrink-0 animate-pulse rounded-card bg-glass"
              style={{ width: CARD_W, height: CARD_W }}
            />
          ))}
        </div>
      ) : (
        <ul className="flex gap-3 overflow-x-auto pb-1">
          {picks.map((p) => (
            <li key={p.itunesId} className="shrink-0" style={{ width: CARD_W }}>
              <button
                type="button"
                onMouseEnter={() => primeOnlinePick(toPick(p))}
                onFocus={() => primeOnlinePick(toPick(p))}
                onClick={() => void playOnlinePick(toPick(p))}
                className="group w-full text-left"
              >
                <CoverArt
                  track={p}
                  size={CARD_W}
                  className="transition group-hover:opacity-80"
                />
                <p className="mt-2 truncate text-13 text-ink-1">{p.title}</p>
                <p className="truncate text-12 text-ink-3">{p.artist}</p>
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
