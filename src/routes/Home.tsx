import { useEffect, useMemo } from "react";
import { formatDuration } from "@/lib/format";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlaylistStore } from "@/store/playlistStore";
import { usePlayerStore } from "@/store/playerStore";

/** Home (P9.6 fix #4 — was a placeholder): resume the queue, jump back
    into recently added tracks, shuffle the library. All data comes from
    stores already hydrated at boot; no extra commands. */

export default function HomePage() {
  const tracks = useLibraryStore((s) => s.tracks);
  const playlists = usePlaylistStore((s) => s.playlists);
  const loadPlaylists = usePlaylistStore((s) => s.load);
  const queue = usePlayerStore((s) => s.queue);
  const currentTrackId = usePlayerStore((s) => s.currentTrackId);
  const isPlaying = usePlayerStore((s) => s.isPlaying);

  useEffect(() => {
    void loadPlaylists();
  }, [loadPlaylists]);

  const current = useMemo(
    () => tracks.find((t) => t.id === currentTrackId) ?? null,
    [tracks, currentTrackId],
  );
  const recent = useMemo(
    () =>
      [...tracks]
        .sort((a, b) => (a.addedAt < b.addedAt ? 1 : -1))
        .slice(0, 6),
    [tracks],
  );

  function playFrom(id: number) {
    const ids = recent.map((t) => t.id);
    usePlayerStore.getState().playQueue(ids, ids.indexOf(id));
  }

  function shuffleAll() {
    if (tracks.length === 0) return;
    const ids = tracks.map((t) => t.id);
    for (let i = ids.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [ids[i], ids[j]] = [ids[j], ids[i]];
    }
    usePlayerStore.getState().playQueue(ids, 0);
  }

  function resume() {
    if (queue.length === 0) return;
    usePlayerStore.getState().setPlaying(true);
  }

  return (
    <section className="page flex flex-col gap-6 overflow-y-auto">
      <header className="flex items-center justify-between">
        <h1 className="text-28 font-display">Home</h1>
        <button
          type="button"
          onClick={shuffleAll}
          disabled={tracks.length === 0}
          className="rounded-ctrl bg-accent px-4 py-2 text-14 font-medium text-[color:var(--on-accent)] transition hover:opacity-90 disabled:opacity-50"
        >
          Shuffle all
        </button>
      </header>

      <p className="text-13 text-ink-3">
        {tracks.length} track{tracks.length === 1 ? "" : "s"} ·{" "}
        {playlists.length} playlist{playlists.length === 1 ? "" : "s"} ·
        press <span className="text-ink-2">Ctrl+K</span> to search
      </p>

      {current && (
        <div className="rounded-card border border-stroke bg-glass p-4">
          <h2 className="mb-2 text-11 font-medium uppercase tracking-wide text-ink-3">
            Continue listening
          </h2>
          <div className="flex items-center justify-between gap-3">
            <div className="min-w-0">
              <p className="truncate text-15 text-ink-1">
                {current.artist} — {current.title}
              </p>
              {current.album && (
                <p className="truncate text-12 text-ink-3">{current.album}</p>
              )}
            </div>
            <button
              type="button"
              onClick={resume}
              disabled={isPlaying}
              className="rounded-ctrl border border-stroke px-4 py-2 text-14 text-ink-2 transition hover:bg-glass disabled:opacity-50"
            >
              {isPlaying ? "Playing" : "Resume"}
            </button>
          </div>
        </div>
      )}

      {recent.length > 0 && (
        <div>
          <h2 className="mb-2 text-11 font-medium uppercase tracking-wide text-ink-3">
            Recently added
          </h2>
          <ul className="flex flex-col gap-1">
            {recent.map((track) => (
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
        </div>
      )}

      {tracks.length === 0 && (
        <p className="text-ink-3">
          Nothing here yet — scan a folder in the Library to get started.
        </p>
      )}
    </section>
  );
}
