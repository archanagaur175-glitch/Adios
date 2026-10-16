import { useCallback, useState } from "react";
import { addPickToPlaylist, pickFromRow, primeOnlinePick } from "@/lib/onlinePlay";
import type { MenuAnchor } from "./AddToPlaylistMenu";
import type { ResultRow } from "./SearchResults";

/** P9.9: the "+ Add to playlist" flow for Ctrl+K rows. The menu opens on
    intent and PRIMES the pick immediately (single-flight resolve — the
    eventual add awaits the same flight), then the add is OPTIMISTIC: the
    menu closes on click and the row wears the progress badges
    ("Resolving…" → "Added ✓") while resolve→persist→link finish in the
    background; a failure clears the badge (the toast explains why).
    Owns the menu state + badges so SearchOverlay only wires callbacks. */

interface MenuState {
  row: ResultRow;
  anchor: MenuAnchor;
}

const ADDED_FLASH_MS = 1600;

export function useAddMenu() {
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [pickingKey, setPickingKey] = useState<string | null>(null);
  const [addedKey, setAddedKey] = useState<string | null>(null);

  /** Opens the portal menu anchored to the "+ " button and primes the
      pick right away — resolving while the user browses playlists. */
  function openMenu(row: ResultRow, button: HTMLElement): void {
    const r = button.getBoundingClientRect();
    primeOnlinePick(pickFromRow(row));
    setMenu({
      row,
      anchor: { left: r.left, top: r.top, bottom: r.bottom, right: r.right },
    });
  }

  const closeMenu = useCallback(() => setMenu(null), []);

  /** Optimistic add: close the menu NOW, badge the row, finish the
      resolve→persist→link chain in the background. */
  function choose(playlistId: number): void {
    if (!menu) return;
    const row = menu.row;
    setMenu(null);
    setPickingKey(row.key);
    void addPickToPlaylist(pickFromRow(row), playlistId).then((ok) => {
      setPickingKey((k) => (k === row.key ? null : k));
      if (!ok) return;
      setAddedKey(row.key);
      window.setTimeout(() => {
        setAddedKey((k) => (k === row.key ? null : k));
      }, ADDED_FLASH_MS);
    });
  }

  const beginPick = (key: string): void => setPickingKey(key);
  const endPick = (key: string): void =>
    setPickingKey((k) => (k === key ? null : k));

  return { menu, pickingKey, addedKey, openMenu, closeMenu, choose, beginPick, endPick };
}
