import { useEffect, useMemo, useState } from "react";
import { inTauri } from "@/lib/api";
import { searchItunes } from "@/lib/onlineApi";
import { playOnlinePick, primeOnlinePick } from "@/lib/onlinePlay";
import {
  findLibraryMatch,
  normalizeMeta,
  planPicks,
  type RecCandidate,
} from "@/lib/recommend";
import { startPicksPlay, toPick } from "@/lib/recQueue";
import { seedQueries } from "@/lib/taste";
import { useLibraryStore } from "@/store/libraryStore";
import { CoverArt } from "@/components/common/CoverArt";

/** P16 section A: "Picked for you" — taste-seeded iTunes search (cached by
    the Rust meta_cache, so first load pays for all later ones), candidates
    already in the library are dropped (Library owns those); cards stream on
    click, Play-all runs the recQueue pick session. Hides itself outside
    Tauri, while there's no history, or when the search fails/returns
    nothing (silent — Home must never hard-fail on the network). */

const PICK_COUNT = 10;
const SEED_COUNT = 2;
const CARD_W = 144;

export function PickedForYou() {
  const tracks = useLibraryStore((s) => s.tracks);
  const seeds = useMemo(
    () => (inTauri ? seedQueries(tracks, Date.now(), SEED_COUNT) : []),
    [tracks],
  );
  const [picks, setPicks] = useState<RecCandidate[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (seeds.length === 0) {
      setPicks([]);
      return;
    }
    let cancelled = false;
    setLoading(true);
    void (async () => {
      const groups: RecCandidate[][] = [];
      for (const seed of seeds) {
        try {
          groups.push(await searchItunes(seed));
        } catch {
          // failed seed → fewer groups; keep going (all-fail → hide below)
        }
      }
      if (cancelled) return;
      const exclude = new Set(
        tracks.map((t) => normalizeMeta(t.title, t.artist)),
      );
      // Library owns its rows (identity `itunes:<id>` AND normalized meta).
      const fresh = groups.map((g) =>
        g.filter((c) => findLibraryMatch(c, tracks) == null),
      );
      setPicks(planPicks(fresh, exclude, PICK_COUNT));
      setLoading(false);
    })().catch(() => {
      if (cancelled) return;
      setPicks([]);
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [seeds, tracks]);

  if (seeds.length === 0) return null;
  if (!loading && picks.length === 0) return null;

  return (
    <section>
      <header className="mb-3 flex items-end justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-18 font-display">Picked for you</h2>
          <p className="text-12 text-ink-3">From the artists you play most</p>
        </div>
        <button
          type="button"
          onClick={() => startPicksPlay(picks)}
          disabled={picks.length === 0}
          className="shrink-0 rounded-ctrl border border-stroke px-4 py-2 text-14 text-ink-2 transition hover:bg-glass disabled:opacity-50"
        >
          Play all
        </button>
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
