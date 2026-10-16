import { useCallback, useEffect, useState } from "react";
import { LibraryTabs, type LibraryTab } from "@/components/library/LibraryTabs";
import { TrackList } from "@/components/library/TrackList";
import { useScanEnrich } from "@/components/library/useScanEnrich";
import { inTauri } from "@/lib/api";
import { formatDuration } from "@/lib/format";
import { listPlayHistory } from "@/lib/historyApi";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import type { Track } from "@/lib/types";

/** P10: Library = Downloads (local files) + History (all plays, most
    recent first). Scan/Enrich act on the local library, so they live on
    the Downloads tab only. */
export default function LibraryPage() {
  const tracks = useLibraryStore((s) => s.tracks);
  const [tab, setTab] = useState<LibraryTab>("downloads");
  const [history, setHistory] = useState<Track[]>([]);
  const { summary, enrichNote, scanning, enriching, error, refresh, runEnrich, handleScan } =
    useScanEnrich();

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const refreshHistory = useCallback(async () => {
    try {
      setHistory(await listPlayHistory());
    } catch {
      /* best-effort — the tab falls back to its empty state */
    }
  }, []);

  useEffect(() => {
    if (tab === "history") void refreshHistory();
  }, [tab, refreshHistory]);

  const downloads = tracks.filter((t) => t.source === "local");

  function playFrom(list: Track[], trackId: number) {
    usePlayerStore
      .getState()
      .playQueue(list.map((t) => t.id), list.findIndex((t) => t.id === trackId));
  }

  return (
    <section className="page flex h-full flex-col gap-4">
      <header className="flex items-center justify-between">
        <h1 className="text-28 font-display">Library</h1>
        <LibraryTabs value={tab} onChange={setTab} />
      </header>

      {tab === "downloads" ? (
        <>
          <div className="flex items-center justify-end gap-2">
            <button
              type="button"
              onClick={() => void runEnrich()}
              disabled={scanning || enriching || !inTauri || downloads.length === 0}
              className="rounded-ctrl border border-stroke px-4 py-2 text-14 text-ink-2 transition hover:bg-glass disabled:opacity-50"
            >
              {enriching ? "Enriching…" : "Enrich"}
            </button>
            <button
              type="button"
              onClick={() => void handleScan()}
              disabled={scanning || enriching || !inTauri}
              className="rounded-ctrl bg-accent px-4 py-2 text-14 font-medium text-[color:var(--on-accent)] transition hover:opacity-90 disabled:opacity-50"
            >
              {scanning ? "Scanning…" : enriching ? "Enriching…" : "Scan folder"}
            </button>
          </div>

          {error && <p className="text-danger text-14">{error}</p>}

          {summary && (
            <p className="text-ink-3 text-14 tabular">
              Added {summary.added} · skipped {summary.skipped} · failed{" "}
              {summary.failed} · {summary.elapsedMs} ms — {summary.folder}
            </p>
          )}

          {enrichNote && (
            <p className="text-ink-3 text-14 tabular">
              Enriched: matched {enrichNote.matched} · covers {enrichNote.covers} ·
              no match {enrichNote.noMatch} · failed {enrichNote.failed} ·{" "}
              {enrichNote.elapsedMs} ms
            </p>
          )}

          {downloads.length === 0 ? (
            <p className="text-ink-3">
              {inTauri
                ? "No local tracks yet — scan a folder to build your library."
                : "Library is available inside the Adios app."}
            </p>
          ) : (
            <TrackList tracks={downloads} onPlay={(id) => playFrom(downloads, id)} />
          )}
        </>
      ) : history.length === 0 ? (
        <p className="text-ink-3">
          Nothing played yet — tracks appear here once you play them.
        </p>
      ) : (
        <TrackList
          tracks={history}
          onPlay={(id) => playFrom(history, id)}
          right={(t) => `${formatDuration(t.durationMs)} · ${t.playCount}×`}
        />
      )}
    </section>
  );
}
