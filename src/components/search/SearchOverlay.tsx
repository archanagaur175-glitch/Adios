/** Ctrl+K search overlay (P5.3): local FTS5 results arrive instantly,
    MusicBrainz tracks/albums/artists append after the 275 ms debounce (D12).
    Arrow keys walk the flattened grouped list; Enter plays local picks, and
    (P9.5) MB track rows stream via the resolve_query chain. */

import { useEffect, useMemo, useRef, useState } from "react";
import { playOnlinePick } from "@/lib/onlinePlay";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";
import { useSearchStore } from "@/store/searchStore";
import { useSearchLayers } from "./useSearchLayers";
import {
  buildGroups,
  flattenGroups,
  SearchResultsList,
  type ResultRow,
} from "./SearchResults";

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
  const [pickingKey, setPickingKey] = useState<string | null>(null);

  useSearchLayers(query);

  const groups = useMemo(
    () => buildGroups({ localResults, onlineTracks, onlineAlbums, onlineArtists }),
    [localResults, onlineTracks, onlineAlbums, onlineArtists],
  );
  const rows = useMemo(() => flattenGroups(groups), [groups]);

  useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  /** P9.5: MusicBrainz track row → resolve_query chain → transient stream
      track → play (logic in lib/onlinePlay.ts; row shows "Resolving…"). */
  async function playOnline(row: ResultRow): Promise<void> {
    if (pickingRef.current || !row.mbid) return;
    pickingRef.current = true;
    setPickingKey(row.key);
    try {
      const started = await playOnlinePick({
        mbid: row.mbid,
        title: row.title,
        artist: row.sub,
        durationMs: row.durationMs,
      });
      if (started) useSearchStore.getState().setOpen(false);
    } finally {
      pickingRef.current = false;
      setPickingKey(null);
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
      message: "Album/artist matching ships with import enrichment (P5.4)",
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
          className="border-b border-stroke bg-transparent px-4 py-3.5 text-15 text-ink-1 placeholder:text-ink-3"
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
            pickingKey={pickingKey}
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
