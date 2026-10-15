import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { inTauri, listTracks, scanFolder, type ScanSummary } from "@/lib/api";
import { formatDuration } from "@/lib/format";
import type { Track } from "@/lib/types";

export default function LibraryPage() {
  const [tracks, setTracks] = useState<Track[]>([]);
  const [summary, setSummary] = useState<ScanSummary | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setTracks(await listTracks());
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function handleScan() {
    setError(null);
    const folder = await open({ directory: true });
    if (typeof folder !== "string") return;
    setScanning(true);
    try {
      const result = await scanFolder(folder);
      if (result) setSummary(result);
      await refresh();
    } catch (scanError) {
      setError(String(scanError));
    } finally {
      setScanning(false);
    }
  }

  return (
    <section className="page flex h-full flex-col gap-4">
      <header className="flex items-center justify-between">
        <h1 className="text-28 font-display">Library</h1>
        <button
          type="button"
          onClick={() => void handleScan()}
          disabled={scanning || !inTauri}
          className="rounded-ctrl bg-accent px-4 py-2 text-14 font-medium text-white transition hover:opacity-90 disabled:opacity-50"
        >
          {scanning ? "Scanning…" : "Scan folder"}
        </button>
      </header>

      {error && <p className="text-danger text-14">{error}</p>}

      {summary && (
        <p className="text-ink-3 text-14 tabular">
          Added {summary.added} · skipped {summary.skipped} · failed{" "}
          {summary.failed} · {summary.elapsedMs} ms — {summary.folder}
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
              className="grid grid-cols-[1fr_auto] items-center gap-3 rounded-card px-3 py-2 transition hover:bg-glass"
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
