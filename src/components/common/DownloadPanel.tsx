import { useEffect } from "react";
import { useDownloadStore } from "@/store/downloadStore";
import { useLibraryStore } from "@/store/libraryStore";

/** P14: persistent download status panel (bottom-right, above the player
    bar). Shows one row per active download with a progress bar, then a
    short-lived "Download complete" row per finished track (fades after 4 s). */

function formatBytes(n: number): string {
  if (n >= 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  if (n >= 1024) return `${Math.round(n / 1024)} KB`;
  return `${n} B`;
}

export function DownloadPanel() {
  const active = useDownloadStore((s) => s.active);
  const done = useDownloadStore((s) => s.done);
  const tracks = useLibraryStore((s) => s.tracks);

  // Auto-fade each "complete" row 4 s after it appears.
  useEffect(() => {
    const timers = done.map((d) =>
      setTimeout(() => useDownloadStore.getState().clearDone(d.trackId), 4000),
    );
    return () => timers.forEach(clearTimeout);
  }, [done]);

  const entries = Object.values(active);
  if (entries.length === 0 && done.length === 0) return null;

  return (
    <div
      className="fixed right-4 z-40 flex w-72 flex-col gap-2"
      style={{ bottom: "calc(var(--space-3) + 72px + var(--space-3))" }}
      aria-label="Download status"
    >
      {entries.map((p) => {
        const title = tracks.find((t) => t.id === p.trackId)?.title ?? `Track ${p.trackId}`;
        const pct = p.total ? Math.min(100, Math.round((p.received / p.total) * 100)) : null;
        return (
          <div
            key={p.trackId}
            role="status"
            className="rounded-card border border-stroke bg-glass px-3 py-2 shadow-lg"
          >
            <p className="truncate text-12 text-ink-1">Downloading “{title}”…</p>
            <div className="mt-1.5 h-1 overflow-hidden rounded-pill bg-glass-strong">
              <div
                className="h-full rounded-pill bg-accent transition-[width] duration-200"
                style={{ width: pct != null ? `${pct}%` : "100%", opacity: pct != null ? 1 : 0.4 }}
              />
            </div>
            <p className="mt-1 text-12 tabular text-ink-3">
              {pct != null ? `${pct}% · ` : ""}
              {formatBytes(p.received)}
              {p.total != null ? ` / ${formatBytes(p.total)}` : ""}
            </p>
          </div>
        );
      })}
      {done.map((d) => (
        <div
          key={`done-${d.trackId}`}
          role="status"
          className="rounded-card border border-stroke bg-glass px-3 py-2 text-14 text-ink-1 shadow-lg"
        >
          ✓ Download complete — “{d.title}”
        </div>
      ))}
    </div>
  );
}
