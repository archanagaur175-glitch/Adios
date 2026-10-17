import { useCallback, useEffect, useState } from "react";
import { inTauri } from "@/lib/api";
import { listCachedTracks } from "@/lib/sessionApi";
import { formatDuration } from "@/lib/format";
import { CoverArt } from "@/components/common/CoverArt";
import { usePlayerStore } from "@/store/playerStore";
import type { Track } from "@/lib/types";

/** Cached view (P8.3): remote tracks with a disk-cache entry — the
    offline-playable slice of the library. */
export default function CachedPage() {
  const [tracks, setTracks] = useState<Track[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setTracks(await listCachedTracks());
    } catch (err) {
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  function play(trackId: number) {
    usePlayerStore
      .getState()
      .playQueue(tracks.map((t) => t.id), tracks.findIndex((t) => t.id === trackId));
  }

  return (
    <section className="page flex h-full flex-col gap-4">
      <header className="flex items-center justify-between">
        <h1 className="text-28 font-display">Cached</h1>
        <span className="text-12 tabular text-ink-3" data-testid="cached-count">
          {tracks.length} track{tracks.length === 1 ? "" : "s"}
        </span>
      </header>

      {error && <p className="text-danger text-14">{error}</p>}

      {tracks.length === 0 ? (
        <p className="text-ink-3">
          {inTauri
            ? "Nothing cached yet — remote tracks appear here once they have been played."
            : "The cached view is available inside the Adios app."}
        </p>
      ) : (
        <ul className="flex flex-col gap-1 overflow-y-auto">
          {tracks.map((track) => (
            <li
              key={track.id}
              onClick={() => play(track.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter") play(track.id);
              }}
              tabIndex={0}
              role="button"
              className="grid cursor-pointer grid-cols-[auto_1fr_auto] items-center gap-3 rounded-card px-3 py-2 transition hover:bg-glass focus-visible:bg-glass"
            >
              <CoverArt track={track} size={36} />
              <div className="min-w-0">
                <p className="truncate text-14 text-ink-1">
                  {track.artist} — {track.title}
                </p>
                {track.album && <p className="truncate text-12 text-ink-3">{track.album}</p>}
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
