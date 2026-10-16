import { useEffect, useState } from "react";
import { searchLocal } from "@/lib/api";
import { addPlaylistTrack } from "@/lib/playlistApi";
import { formatDuration } from "@/lib/format";
import { useUiStore } from "@/store/uiStore";
import type { Track } from "@/lib/types";

interface Props {
  playlistId: number;
  /** Current member ids — excluded from the picker. */
  memberIds: Set<number>;
  /** Refresh the parent detail after a successful add. */
  onAdded: () => void;
}

/** Library search + one-click add (P8.2). */
export function AddTracks({ playlistId, memberIds, onAdded }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Track[]>([]);
  const pushToast = useUiStore((s) => s.pushToast);

  useEffect(() => {
    let cancelled = false;
    const q = query.trim();
    if (!q) {
      setResults([]);
      return;
    }
    void searchLocal(q)
      .then((tracks) => {
        if (!cancelled) setResults(tracks);
      })
      .catch(() => {
        if (!cancelled) setResults([]);
      });
    return () => {
      cancelled = true;
    };
  }, [query]);

  async function add(track: Track) {
    try {
      const ok = await addPlaylistTrack(playlistId, track.id);
      if (ok) {
        onAdded();
      } else {
        pushToast({ message: "Already in this playlist", tone: "info" });
      }
    } catch (err) {
      pushToast({ message: String(err), tone: "danger" });
    }
  }

  const fresh = results.filter((t) => !memberIds.has(t.id));

  return (
    <div className="flex flex-col gap-2" data-testid="add-tracks">
      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search library to add tracks…"
        aria-label="Search library to add tracks"
        className="rounded-ctrl border border-stroke bg-transparent px-3 py-2 text-14 text-ink-1 outline-none placeholder:text-ink-3 focus:border-accent"
      />
      {query.trim() && results.length > 0 && fresh.length === 0 && (
        <p className="text-12 text-ink-3">All matches are already in this playlist.</p>
      )}
      <ul className="flex max-h-64 flex-col gap-1 overflow-y-auto">
        {fresh.map((track) => (
          <li key={track.id}>
            <button
              type="button"
              onClick={() => void add(track)}
              className="flex w-full items-center justify-between rounded-card px-3 py-2 text-left transition hover:bg-glass"
            >
              <span className="min-w-0 truncate text-14 text-ink-1">
                {track.artist} — {track.title}
              </span>
              <span className="ml-3 shrink-0 text-12 tabular text-ink-3">
                {formatDuration(track.durationMs)} · add
              </span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
