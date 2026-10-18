import { useCallback, useEffect, useState } from "react";
import { LibraryTabs, type LibraryTab } from "@/components/library/LibraryTabs";
import { TrackList } from "@/components/library/TrackList";
import { FolderPicker } from "@/components/library/FolderPicker";
import { useScan } from "@/components/library/useScan";
import { inTauri } from "@/lib/api";
import { formatDuration } from "@/lib/format";
import { listPlayHistory } from "@/lib/historyApi";
import { refreshCachedIds } from "@/lib/downloadApi";
import { useDownloadStore } from "@/store/downloadStore";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import type { Track } from "@/lib/types";

/** P11/P12: Library = Downloads (local files + P13 pinned stream
    downloads) + History (all plays, most recent first). Scan lives on the
    Downloads tab; import enrichment was removed in P12. */
export default function LibraryPage() {
  const tracks = useLibraryStore((s) => s.tracks);
  const cachedIds = useDownloadStore((s) => s.cachedIds);
  const [tab, setTab] = useState<LibraryTab>("downloads");
  const [history, setHistory] = useState<Track[]>([]);
  const {
    summary,
    scanning,
    error,
    refresh,
    handleScan,
    picking,
    pickStart,
    pickFolder,
    cancelPick,
  } = useScan();

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const ids = tracks.filter((t) => t.source === "stream").map((t) => t.id);
    void refreshCachedIds(ids);
  }, [tracks]);

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
  const pinned = tracks.filter((t) => t.source === "stream" && cachedIds[t.id]);

  function playFrom(list: Track[], trackId: number) {
    usePlayerStore
      .getState()
      .playQueue(list.map((t) => t.id), list.findIndex((t) => t.id === trackId));
  }

  return (
    <section className="page flex h-full flex-col gap-4 overflow-y-auto">
      <header className="flex items-center justify-between">
        <h1 className="text-28 font-display">Library</h1>
        <LibraryTabs value={tab} onChange={setTab} />
      </header>

      {tab === "downloads" ? (
        <>
          <div className="flex items-center justify-end gap-2">
            <button
              type="button"
              onClick={() => void handleScan()}
              disabled={scanning || !inTauri}
              className="rounded-ctrl bg-accent px-4 py-2 text-14 font-medium text-[color:var(--on-accent)] transition hover:opacity-90 disabled:opacity-50"
            >
              {scanning ? "Scanning…" : "Scan folder"}
            </button>
          </div>

          {error && <p className="text-danger text-14">{error}</p>}

          {summary && (
            <p className="text-ink-3 text-14 tabular">
              Added {summary.added} · skipped {summary.skipped} · failed{" "}
              {summary.failed} · {summary.elapsedMs} ms — {summary.folder}
            </p>
          )}

          {picking && (
            <FolderPicker
              start={pickStart}
              onPick={pickFolder}
              onCancel={cancelPick}
            />
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

          {pinned.length > 0 && (
            <div>
              <h2 className="mb-2 text-11 font-medium uppercase tracking-wide text-ink-3">
                Pinned downloads
              </h2>
              <TrackList
                tracks={pinned}
                onPlay={(id) => playFrom(pinned, id)}
                right={(t) => `${formatDuration(t.durationMs)} · on disk`}
              />
            </div>
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
