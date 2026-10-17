import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router";
import { CoverArt } from "@/components/common/CoverArt";
import { savePicksToPlaylist } from "@/lib/playlistSave";
import { moodById } from "@/lib/moods";
import { searchItunes } from "@/lib/onlineApi";
import { playOnlinePick, primeOnlinePick } from "@/lib/onlinePlay";
import { findLibraryMatch, planPicks, type RecCandidate } from "@/lib/recommend";
import { startPicksPlay, toPick } from "@/lib/recQueue";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlaylistStore } from "@/store/playlistStore";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";

/** P16 (B): `#/moods/:id` — the mood's iTunes query runs ON OPEN (Rust
    search cache). Rows play in place (library fast-path → playOnlinePick),
    Play/Shuffle run the recQueue pick session (first pick replaces the
    queue, the rest trickle), Save walks the sequential save flow with
    an inline `Saving n/total…` readout. Unknown id → honest dead end. */

/** Matches the plan's `Saving 4/12…` readout — 12 rows keeps a full
    sequential save within a reasonable wait (each unowned pick costs one
    resolver chain). */
const ROW_COUNT = 12;

export default function MoodPage() {
  const { id } = useParams();
  const mood = useMemo(() => moodById(id), [id]);
  const loadPlaylists = usePlaylistStore((s) => s.load);
  const pushToast = useUiStore((s) => s.pushToast);
  const [picks, setPicks] = useState<RecCandidate[]>([]);
  const [loading, setLoading] = useState(false);
  /** 0 = idle; >0 = items completed so far (button shows the readout). */
  const [saving, setSaving] = useState(0);

  useEffect(() => {
    if (!mood) return;
    let cancelled = false;
    setLoading(true);
    setPicks([]);
    void (async () => {
      try {
        const rows = await searchItunes(mood.query);
        if (!cancelled) setPicks(planPicks([rows], new Set(), ROW_COUNT));
      } catch {
        if (!cancelled) setPicks([]);
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [mood]);

  function rowPlay(p: RecCandidate) {
    // Owned already → instant local play; else the full online pick flow.
    const fast = findLibraryMatch(p, useLibraryStore.getState().tracks);
    if (fast != null) {
      usePlayerStore.getState().playQueue([fast], 0);
      return;
    }
    void playOnlinePick(toPick(p));
  }

  function runList(shuffle: boolean) {
    if (picks.length === 0) return;
    const list = [...picks];
    if (shuffle) {
      for (let i = list.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1));
        [list[i], list[j]] = [list[j], list[i]];
      }
    }
    startPicksPlay(list);
  }

  async function save() {
    if (!mood || saving > 0 || picks.length === 0) return;
    setSaving(1);
    const res = await savePicksToPlaylist(mood.name, picks, (done) => setSaving(done));
    setSaving(0);
    if (!res || res.added === 0) {
      pushToast({ message: `Nothing saved for “${mood.name}” — no streams found`, tone: "danger" });
      return;
    }
    void loadPlaylists();
    const note = res.skipped > 0 ? ` (${res.skipped} skipped)` : "";
    pushToast({
      message: `Saved ${res.added} track${res.added === 1 ? "" : "s"} to “${res.name}”${note}`,
      tone: "info",
    });
  }

  if (!mood) {
    return (
      <section className="page">
        <p className="text-ink-3">
          Unknown mood. <Link to="/" className="text-accent">Back to Home</Link>
        </p>
      </section>
    );
  }

  return (
    <section className="page flex flex-col gap-6 overflow-y-auto">
      <header
        className="flex flex-wrap items-start justify-between gap-3 rounded-card p-4"
        style={{ background: mood.gradient }}
      >
        <div className="min-w-0">
          <h1 className="text-28 font-display text-white/95">{mood.name}</h1>
          <p className="text-13 text-white/70">Top matches for “{mood.query}”</p>
        </div>
        <div className="flex shrink-0 flex-wrap gap-2">
          <button
            type="button"
            onClick={() => runList(false)}
            disabled={picks.length === 0}
            className="rounded-ctrl bg-white/90 px-4 py-2 text-14 font-medium text-black transition hover:bg-white disabled:opacity-50"
          >
            ▶ Play
          </button>
          <button
            type="button"
            onClick={() => runList(true)}
            disabled={picks.length === 0}
            className="rounded-ctrl border border-white/40 px-4 py-2 text-14 text-white transition hover:bg-white/10 disabled:opacity-50"
          >
            Shuffle
          </button>
          <button
            type="button"
            onClick={() => void save()}
            disabled={saving > 0 || picks.length === 0}
            className="rounded-ctrl border border-white/40 px-4 py-2 text-14 text-white transition hover:bg-white/10 disabled:opacity-70"
          >
            {saving > 0 ? `Saving ${saving}/${picks.length}…` : "Save to playlists"}
          </button>
        </div>
      </header>

      {loading ? (
        <div className="flex flex-col gap-2" aria-hidden="true">
          {[0, 1, 2, 3, 4, 5].map((i) => (
            <div key={i} className="h-14 animate-pulse rounded-card bg-glass" />
          ))}
        </div>
      ) : picks.length === 0 ? (
        <p className="text-ink-3">Nothing found for this mood — check your connection.</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {picks.map((p) => (
            <li key={p.itunesId ?? `${p.title}|${p.artist}`}>
              <button
                type="button"
                onMouseEnter={() => primeOnlinePick(toPick(p))}
                onFocus={() => primeOnlinePick(toPick(p))}
                onClick={() => rowPlay(p)}
                className="flex w-full items-center gap-3 rounded-card border border-stroke bg-glass p-2 text-left transition hover:bg-white/5"
              >
                <CoverArt track={p} size={44} />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-14 text-ink-1">{p.title}</p>
                  <p className="truncate text-12 text-ink-3">{p.artist}</p>
                </div>
                {p.genre && (
                  <span className="hidden shrink-0 text-11 text-ink-3 sm:block">{p.genre}</span>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
