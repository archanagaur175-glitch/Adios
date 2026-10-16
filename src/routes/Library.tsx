import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  enrichTracks,
  inTauri,
  listTracks,
  scanFolder,
  type EnrichSummary,
  type ScanSummary,
} from "@/lib/api";
import { formatDuration } from "@/lib/format";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";

export default function LibraryPage() {
  const tracks = useLibraryStore((s) => s.tracks);
  const [summary, setSummary] = useState<ScanSummary | null>(null);
  const [enrichNote, setEnrichNote] = useState<EnrichSummary | null>(null);
  const [scanning, setScanning] = useState(false);
  const [enriching, setEnriching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    useLibraryStore.getState().setTracks(await listTracks());
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  /** Enrich unmatched rows — batches of ≤ 50 until the well runs dry. */
  async function runEnrich(): Promise<EnrichSummary | null> {
    if (!inTauri) return null;
    setEnriching(true);
    try {
      let total: EnrichSummary = { matched: 0, noMatch: 0, failed: 0, covers: 0, elapsedMs: 0 };
      for (;;) {
        const batch = await enrichTracks();
        total = {
          matched: total.matched + batch.matched,
          noMatch: total.noMatch + batch.noMatch,
          failed: total.failed + batch.failed,
          covers: total.covers + batch.covers,
          elapsedMs: total.elapsedMs + batch.elapsedMs,
        };
        if (batch.matched + batch.noMatch + batch.failed < 50) break;
      }
      setEnrichNote(total);
      await refresh();
      return total;
    } catch (enrichError) {
      setError(String(enrichError));
      return null;
    } finally {
      setEnriching(false);
    }
  }

  async function handleScan() {
    setError(null);
    const folder = await open({ directory: true });
    if (typeof folder !== "string") return;
    setScanning(true);
    try {
      const result = await scanFolder(folder);
      if (result) setSummary(result);
      await refresh();
      // Enrich on import (P5.4): only when the scan actually added rows.
      if (result && result.added > 0) await runEnrich();
    } catch (scanError) {
      setError(String(scanError));
    } finally {
      setScanning(false);
    }
  }

  function playFrom(trackId: number) {
    usePlayerStore
      .getState()
      .playQueue(tracks.map((t) => t.id), tracks.findIndex((t) => t.id === trackId));
  }

  return (
    <section className="page flex h-full flex-col gap-4">
      <header className="flex items-center justify-between">
        <h1 className="text-28 font-display">Library</h1>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => void runEnrich()}
            disabled={scanning || enriching || !inTauri || tracks.length === 0}
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
      </header>

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

      {tracks.length === 0 ? (
        <p className="text-ink-3">
          {inTauri
            ? "No tracks yet — scan a folder to build your library."
            : "Library is available inside the Adios app."}
        </p>
      ) : (
        <ul className="flex flex-col gap-1 overflow-y-auto">
          {tracks.map((track) => (
            <li
              key={track.id}
              onClick={() => playFrom(track.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter") playFrom(track.id);
              }}
              tabIndex={0}
              role="button"
              className="grid cursor-pointer grid-cols-[1fr_auto] items-center gap-3 rounded-card px-3 py-2 transition hover:bg-glass focus-visible:bg-glass"
            >
              <div className="min-w-0">
                <p className="truncate text-14 text-ink-1">
                  {track.artist} — {track.title}
                </p>
                {track.album && (
                  <p className="truncate text-12 text-ink-3">{track.album}</p>
                )}
              </div>
              <span className="text-12 tabular text-ink-3">
                {formatDuration(track.durationMs)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
