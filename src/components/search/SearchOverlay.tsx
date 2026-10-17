/** Ctrl+K search overlay (P5.3/P9.5/P9.7): local FTS5 results arrive
    instantly; online tracks (iTunes primary, MB fallback), albums and
    artists append after the 275 ms debounce (D12). Arrow keys walk the
    flattened list; Enter plays local picks; online track rows stream via
    the resolve chain with a hover prime cache, and "+ Add" persists them
    into a playlist. */

import { useEffect, useMemo, useRef } from "react";
import {
  pickFromRow as rowPick,
  playOnlinePick,
  primeOnlinePick,
} from "@/lib/onlinePlay";
import { downloadOnlinePick } from "@/lib/downloadApi";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";
import { useSearchStore } from "@/store/searchStore";
import { useSearchLayers } from "./useSearchLayers";
import { useAutoPrime } from "./useAutoPrime";
import { useAddMenu } from "./useAddMenu";
import { AddToPlaylistMenu } from "./AddToPlaylistMenu";
import {
  buildGroups,
  flattenGroups,
  SearchResultsList,
  type ResultRow,
} from "./SearchResults";

/** Hover dwell before pre-resolving a row (P9.7): long enough to filter
    accidental passes, short enough to feel instant on a deliberate pick. */
const PRIME_DWELL_MS = 200;

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
  const pickingRef = useRef(false);
  const { menu, pickingKey, addedKey, openMenu, closeMenu, choose, beginPick, endPick } =
    useAddMenu();

  useSearchLayers(query);
  useAutoPrime(open, query, onlineTracks);

  const trackLabel = useMemo(
    () =>
      onlineTracks.some((t) => t.itunesId != null)
        ? "iTunes"
        : "MusicBrainz tracks",
    [onlineTracks],
  );
  const groups = useMemo(
    () =>
      buildGroups({ localResults, onlineTracks, onlineAlbums, onlineArtists, trackLabel }),
    [localResults, onlineTracks, onlineAlbums, onlineArtists, trackLabel],
  );
  const rows = useMemo(() => flattenGroups(groups), [groups]);

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  // P9.7 hover pre-resolve: dwell 200 ms on an online track row, then fire
  // the chain in the background (results cached in onlinePlay's spec map).
  const activeRow = rows[activeIndex];
  useEffect(() => {
    if (!open || !activeRow?.mbid) return;
    const t = window.setTimeout(() => primeOnlinePick(rowPick(activeRow)), PRIME_DWELL_MS);
    return () => window.clearTimeout(t);
  }, [open, activeRow]);

  /** Online track row → resolve chain (possibly primed) → persisted stream
      track → play (logic in lib/onlinePlay.ts; row shows "Resolving…"). */
  async function playOnline(row: ResultRow): Promise<void> {
    if (pickingRef.current || !row.mbid) return;
    pickingRef.current = true;
    beginPick(row.key);
    try {
      const started = await playOnlinePick(rowPick(row));
      if (started) useSearchStore.getState().setOpen(false);
    } finally {
      pickingRef.current = false;
      endPick(row.key);
    }
  }

  function pick(row?: ResultRow) {
    if (!row) return;
    if (row.trackId != null) {
      const ids = localResults.map((t) => t.id);
      usePlayerStore.getState().playQueue(ids, ids.indexOf(row.trackId));
      useSearchStore.getState().setOpen(false);
      return;
    }
    if (row.mbid) {
      void playOnline(row);
      return;
    }
    useUiStore.getState().pushToast({
      message: "Album/artist rows can't play yet — pick a track instead",
      tone: "info",
    });
  }

  /** P12: ⭳ on an online row — resolve → persist stream row → pinned download. */
  function download(row: ResultRow) {
    if (!row.mbid) return;
    void downloadOnlinePick(rowPick(row)).then((ok) => {
      if (ok) useUiStore.getState().pushToast({ message: `Downloaded “${row.title}”`, tone: "info" });
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
        className="glass relative flex h-[min(62vh,580px)] w-[min(680px,92vw)] flex-col overflow-hidden rounded-card"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onKeyDown}
        role="dialog"
        aria-label="Search"
      >
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => useSearchStore.getState().setQuery(e.target.value)}
          placeholder="Search your library, iTunes and MusicBrainz…"
          spellCheck={false}
          autoComplete="off"
          className="border-b border-stroke bg-transparent px-4 py-3.5 text-15 text-ink-1 placeholder:text-ink-3"
        />
        <div className="flex-1 overflow-y-auto px-2 pb-2">
          {onlinePending && (
            <p className="px-3 py-1.5 text-12 text-ink-3">Searching online…</p>
          )}
          {rows.length === 0 && !onlinePending && query.trim() && (
            <p className="px-3 py-1.5 text-13 text-ink-3">
              No matches for “{query.trim()}”.
            </p>
          )}
          <SearchResultsList
            groups={groups}
            activeIndex={activeIndex}
            pickingKey={pickingKey}
            addedKey={addedKey}
            onHover={(i) => useSearchStore.getState().setActiveIndex(i)}
            onPick={pick}
            onAdd={openMenu}
            onDownload={download}
          />
        </div>
        {menu && (
          <AddToPlaylistMenu
            pick={rowPick(menu.row)}
            anchor={menu.anchor}
            onClose={closeMenu}
            onChoose={choose}
          />
        )}
        <footer className="border-t border-stroke px-4 py-2 text-12 text-ink-3">
          ↑↓ navigate · ↵ play · + add to playlist · esc close
        </footer>
      </div>
    </div>
  );
}
