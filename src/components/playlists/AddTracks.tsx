import { useEffect, useState } from "react";
import { searchLocal } from "@/lib/api";
import { addPlaylistTrack } from "@/lib/playlistApi";
import { formatDuration } from "@/lib/format";
import { fetchOnlineTracks, type OnlineTrackHit } from "@/components/search/onlineTracks";
import { addPickToPlaylist } from "@/lib/onlinePlay";
import { useUiStore } from "@/store/uiStore";
import type { Track } from "@/lib/types";

interface Props {
  playlistId: number;
  /** Current member ids — excluded from the picker. */
  memberIds: Set<number>;
  /** Refresh the parent detail after a successful add. */
  onAdded: () => void;
}

const ONLINE_DEBOUNCE_MS = 275;

/** P8.2 library search + P9.8 #2: online (iTunes→MB) layer — picking an
    online hit resolves it, persists a 'stream' row and links it into this
    playlist (addPickToPlaylist), same path as the Ctrl+K "+" button. */
export function AddTracks({ playlistId, memberIds, onAdded }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Track[]>([]);
  const [online, setOnline] = useState<OnlineTrackHit[]>([]);
  const [onlinePending, setOnlinePending] = useState(false);
  const pushToast = useUiStore((s) => s.pushToast);

  // Local FTS layer (instant).
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

  // Online layer (debounced, independent of local).
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      setOnline([]);
      setOnlinePending(false);
      return;
    }
    let cancelled = false;
    setOnlinePending(true);
    const t = window.setTimeout(() => {
      fetchOnlineTracks(q)
        .then((rows) => {
          if (!cancelled) setOnline(rows);
        })
        .catch((err) => console.error("[add-tracks] online failed:", err))
        .finally(() => {
          if (!cancelled) setOnlinePending(false);
        });
    }, ONLINE_DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(t);
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

  async function addOnline(hit: OnlineTrackHit) {
    const ok = await addPickToPlaylist(
      {
        mbid: hit.mbid,
        itunesId: hit.itunesId,
        title: hit.title,
        artist: hit.artist,
        durationMs: hit.durationMs,
      },
      playlistId,
    );
    if (ok) onAdded();
  }

  const fresh = results.filter((t) => !memberIds.has(t.id));
  const nothingYet =
    Boolean(query.trim()) && fresh.length === 0 && online.length === 0 && !onlinePending;

  return (
    <div className="flex flex-col gap-2" data-testid="add-tracks">
      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search library and online to add tracks…"
        aria-label="Search library and online to add tracks"
        className="rounded-ctrl border border-stroke bg-transparent px-3 py-2 text-14 text-ink-1 placeholder:text-ink-3 focus:border-accent"
      />
      {nothingYet && (
        <p className="text-12 text-ink-3">All matches are already in this playlist.</p>
      )}
      {onlinePending && fresh.length === 0 && online.length === 0 && (
        <p className="text-12 text-ink-3">Searching online…</p>
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
        {online.length > 0 && (
          <li className="mt-1 border-t border-stroke pt-1 text-11 font-medium uppercase tracking-wide text-ink-3">
            Online
          </li>
        )}
        {online.map((hit) => (
          <li key={hit.mbid}>
            <button
              type="button"
              onClick={() => void addOnline(hit)}
              className="flex w-full items-center justify-between rounded-card px-3 py-2 text-left transition hover:bg-glass"
            >
              <span className="min-w-0 truncate text-14 text-ink-1">
                {hit.artist} — {hit.title}
              </span>
              <span className="ml-3 shrink-0 text-12 tabular text-ink-3">
                {formatDuration(hit.durationMs)} · add
              </span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
