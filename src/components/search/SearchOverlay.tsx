/** Ctrl+K search overlay (P5.3): local FTS5 results arrive instantly,
    MusicBrainz tracks/albums/artists append after the 275 ms debounce (D12).
    Arrow keys walk the flattened grouped list; Enter plays local picks. */

import { useEffect, useMemo, useRef } from "react";
import { searchLocal, searchMetadata } from "@/lib/api";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";
import type { MbArtistItem, MbRecordingItem, MbReleaseItem } from "@/lib/types";
import { useSearchStore } from "@/store/searchStore";
import {
  buildGroups,
  flattenGroups,
  SearchResultsList,
  type ResultRow,
} from "./SearchResults";

const DEBOUNCE_MS = 275;

const creditNames = (
  credit?: { artist?: { name: string } | null }[] | null,
): string =>
  (credit ?? [])
    .map((c) => c.artist?.name)
    .filter((n): n is string => Boolean(n))
    .join(", ");

export function SearchOverlay() {
  const open = useSearchStore((s) => s.open);
  const query = useSearchStore((s) => s.query);
  const onlinePending = useSearchStore((s) => s.onlinePending);
  const activeIndex = useSearchStore((s) => s.activeIndex);
  const localResults = useSearchStore((s) => s.localResults);
  const onlineTracks = useSearchStore((s) => s.onlineTracks);
  const onlineAlbums = useSearchStore((s) => s.onlineAlbums);
  const onlineArtists = useSearchStore((s) => s.onlineArtists);
  const inputRef = useRef<HTMLInputElement>(null);

  // Local layer: query on every keystroke, stale responses dropped.
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      useSearchStore.getState().setLocalResults([]);
      return;
    }
    let cancelled = false;
    searchLocal(q)
      .then((rows) => {
        if (!cancelled) useSearchStore.getState().setLocalResults(rows);
      })
      .catch((err) => console.error("[search] local failed:", err));
    return () => {
      cancelled = true;
    };
  }, [query]);

  // Online layer: 275 ms debounce, three MB kinds in parallel (the client
  // serializes them through the 1-req/s queue), stale runs cancelled.
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      const s = useSearchStore.getState();
      s.setOnlineResults({ tracks: [], albums: [], artists: [] });
      s.setOnlinePending(false);
      return;
    }
    let cancelled = false;
    useSearchStore.getState().setOnlinePending(true);
    const timer = window.setTimeout(() => {
      Promise.all([
        searchMetadata<MbRecordingItem>("recording", q),
        searchMetadata<MbReleaseItem>("release", q),
        searchMetadata<MbArtistItem>("artist", q),
      ])
        .then(([rec, rel, art]) => {
          if (cancelled) return;
          useSearchStore.getState().setOnlineResults({
            tracks: rec.items.map((r) => ({
              mbid: r.id,
              title: r.title,
              artist: creditNames(r.artistCredit),
              durationMs: r.length ?? undefined,
            })),
            albums: rel.items.map((r) => ({
              mbid: r.id,
              title: r.title,
              artist: creditNames(r.artistCredit),
            })),
            artists: art.items.map((r) => ({
              mbid: r.id,
              title: r.name,
              artist: r.disambiguation ?? "",
            })),
          });
        })
        .catch((err) => console.error("[search] musicbrainz failed:", err))
        .finally(() => {
          if (!cancelled) useSearchStore.getState().setOnlinePending(false);
        });
    }, DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [query]);

  const groups = useMemo(
    () => buildGroups({ localResults, onlineTracks, onlineAlbums, onlineArtists }),
    [localResults, onlineTracks, onlineAlbums, onlineArtists],
  );
  const rows = useMemo(() => flattenGroups(groups), [groups]);

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  function pick(row?: ResultRow) {
    if (!row) return;
    if (row.trackId != null) {
      const ids = localResults.map((t) => t.id);
      usePlayerStore.getState().playQueue(ids, ids.indexOf(row.trackId));
      useSearchStore.getState().setOpen(false);
      return;
    }
    useUiStore.getState().pushToast({
      message: "MB matching ships with import enrichment (P5.4)",
      tone: "info",
    });
  }

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      useSearchStore.getState().setOpen(false);
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      if (rows.length === 0) return;
      e.preventDefault();
      const cur = useSearchStore.getState().activeIndex;
      const next =
        e.key === "ArrowDown"
          ? Math.min(cur + 1, rows.length - 1)
          : Math.max(cur - 1, 0);
      useSearchStore.getState().setActiveIndex(next);
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      pick(rows[useSearchStore.getState().activeIndex]);
    }
  }

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-40 flex justify-center bg-black/45 pt-[12vh]"
      onClick={() => useSearchStore.getState().setOpen(false)}
      role="presentation"
    >
      <div
        className="glass flex h-[min(62vh,580px)] w-[min(680px,92vw)] flex-col overflow-hidden rounded-card"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onKeyDown}
        role="dialog"
        aria-label="Search"
      >
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => useSearchStore.getState().setQuery(e.target.value)}
          placeholder="Search your library and MusicBrainz…"
          spellCheck={false}
          autoComplete="off"
          className="border-b border-stroke bg-transparent px-4 py-3.5 text-15 text-ink-1 outline-none placeholder:text-ink-3"
        />
        <div className="flex-1 overflow-y-auto px-2 pb-2">
          {onlinePending && (
            <p className="px-3 py-1.5 text-12 text-ink-3">Searching MusicBrainz…</p>
          )}
          {rows.length === 0 && !onlinePending && query.trim() && (
            <p className="px-3 py-1.5 text-13 text-ink-3">
              No matches for “{query.trim()}”.
            </p>
          )}
          <SearchResultsList
            groups={groups}
            activeIndex={activeIndex}
            onHover={(i) => useSearchStore.getState().setActiveIndex(i)}
            onPick={pick}
          />
        </div>
        <footer className="border-t border-stroke px-4 py-2 text-12 text-ink-3">
          ↑↓ navigate · ↵ play · esc close
        </footer>
      </div>
    </div>
  );
}
