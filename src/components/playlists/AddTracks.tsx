import { useEffect, useRef, useState } from "react";
import { searchLocal } from "@/lib/api";
import { addPlaylistTrack } from "@/lib/playlistApi";
import { formatDuration } from "@/lib/format";
import { CoverArt } from "@/components/common/CoverArt";
import { downloadOnlinePick } from "@/lib/downloadApi";
import { fetchOnlineTracks, type OnlineTrackHit } from "@/components/search/onlineTracks";
import { useAutoPrime } from "@/components/search/useAutoPrime";
import {
  addPickToPlaylist,
  primeOnlinePick,
  type OnlinePick,
} from "@/lib/onlinePlay";
import { useUiStore } from "@/store/uiStore";
import type { Track } from "@/lib/types";

interface Props {
  playlistId: number;
  memberIds: Set<number>; // current member ids — excluded from the picker
  onAdded: () => void; // refresh the parent detail after a successful add
}

const ONLINE_DEBOUNCE_MS = 275;
const PRIME_DWELL_MS = 200; // P9.9: same dwell as the Ctrl+K overlay's hover prime.

/** P8.2 library search + P9.8 #2 online layer + P9.9 priming/optimistic
    add + P12 ⭳ pinned downloads on online rows. */
export function AddTracks({ playlistId, memberIds, onAdded }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Track[]>([]);
  const [online, setOnline] = useState<OnlineTrackHit[]>([]);
  const [onlinePending, setOnlinePending] = useState(false);
  const [addingKey, setAddingKey] = useState<string | null>(null);
  const hoverTimer = useRef<number | undefined>(undefined);
  const pushToast = useUiStore((s) => s.pushToast);
  const hitPick = (h: OnlineTrackHit): OnlinePick => ({
    mbid: h.mbid, itunesId: h.itunesId, title: h.title, artist: h.artist, durationMs: h.durationMs,
  });

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

  // P9.9: prime top results on arrival + the hovered row after a 200 ms dwell.
  useAutoPrime(true, query, online);
  useEffect(() => () => window.clearTimeout(hoverTimer.current), []);
  const primeHovered = (hit: OnlineTrackHit): void => {
    window.clearTimeout(hoverTimer.current);
    hoverTimer.current = window.setTimeout(() => primeOnlinePick(hitPick(hit)), PRIME_DWELL_MS);
  };
  const cancelHover = (): void => window.clearTimeout(hoverTimer.current);

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

  // P9.9 optimistic: "Adding…" badge sync with the click, resolve off-thread.
  function addOnline(hit: OnlineTrackHit) {
    if (addingKey === hit.mbid) return;
    const key = hit.mbid;
    setAddingKey(key);
    void addPickToPlaylist(hitPick(hit), playlistId)
      .then((ok) => {
        if (ok) onAdded();
      })
      .catch((err) => console.error("[add-tracks] add failed:", err))
      .finally(() => setAddingKey((k) => (k === key ? null : k)));
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
              className="flex w-full items-center gap-2 rounded-card px-3 py-2 text-left transition hover:bg-glass"
            >
              <CoverArt track={track} size={32} />
              <span className="min-w-0 flex-1 truncate text-14 text-ink-1">
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
          <li
            key={hit.mbid}
            onMouseEnter={() => primeHovered(hit)}
            onMouseLeave={cancelHover}
            className="flex items-center gap-1"
          >
            <button
              type="button"
              onClick={() => addOnline(hit)}
              className="flex min-w-0 flex-1 items-center gap-2 rounded-card px-3 py-2 text-left transition hover:bg-glass"
            >
              <CoverArt track={hit} size={32} />
              <span className="min-w-0 flex-1 truncate text-14 text-ink-1">
                {hit.artist} — {hit.title}
              </span>
              <span className="ml-3 shrink-0 text-12 tabular text-ink-3">
                {addingKey === hit.mbid
                  ? "Adding…"
                  : `${formatDuration(hit.durationMs)} · add`}
              </span>
            </button>
            <button
              type="button"
              aria-label={`Download “${hit.title}”`}
              onClick={() => void downloadOnlinePick(hitPick(hit))}
              className="shrink-0 rounded-6 px-1.5 py-0.5 text-14 text-ink-3 transition hover:bg-glass hover:text-ink-1"
            >
              ⭳
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
