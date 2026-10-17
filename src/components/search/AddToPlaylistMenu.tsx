/** P9.7/P9.8/P9.9: playlist picker for "+ Add" on online search rows.
    P9.8 #1: portals to document.body with fixed coords — the search
    dialog's overflow-hidden clipped the old absolute menu out of view.
    P9.9: mounting PRIMES the pick (single-flight resolve runs while the
    user browses playlists, so the add is usually instant) and the click
    DELEGATES to the parent via onChoose — the menu closes immediately and
    the row carries the progress badge (optimistic add). */

import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { primeOnlinePick, type OnlinePick } from "@/lib/onlinePlay";
import { listPlaylists } from "@/lib/playlistApi";
import type { PlaylistSummary } from "@/lib/types";

/** Button-rect anchor, captured at click time (page doesn't move under it). */
export interface MenuAnchor {
  left: number;
  top: number;
  bottom: number;
  right: number;
}

const MENU_ESTIMATE_PX = 240;

interface Props {
  /** P15: optional — the player bar adds an existing library track (no
      resolve/prime needed); search rows still pass a pick to prime it. */
  pick?: OnlinePick;
  anchor: MenuAnchor;
  onClose: () => void;
  /** P9.9: parent starts the optimistic add and unmounts this menu. */
  onChoose: (playlistId: number) => void;
}

export function AddToPlaylistMenu({ pick, anchor, onClose, onChoose }: Props) {
  const [playlists, setPlaylists] = useState<PlaylistSummary[]>([]);
  const ref = useRef<HTMLDivElement>(null);

  // P9.9: intent shown the moment "+" is clicked — starts the shared
  // flight (idempotent: an existing prime/cache hit is left alone).
  useEffect(() => {
    if (pick) primeOnlinePick(pick);
  }, [pick]);

  useEffect(() => {
    listPlaylists()
      .then(setPlaylists)
      .catch((err) => console.error("[search] playlist_list failed:", err));
    const onDocClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    // Deferred so the same-click that opened the menu doesn't close it.
    const t = window.setTimeout(() => document.addEventListener("mousedown", onDocClick), 0);
    return () => {
      window.clearTimeout(t);
      document.removeEventListener("mousedown", onDocClick);
    };
  }, [onClose]);

  const flipUp = anchor.bottom + MENU_ESTIMATE_PX > window.innerHeight;
  const style: React.CSSProperties = {
    position: "fixed",
    right: Math.max(8, window.innerWidth - anchor.right),
    ...(flipUp
      ? { bottom: Math.max(8, window.innerHeight - anchor.top + 4) }
      : { top: anchor.bottom + 4 }),
  };

  return createPortal(
    <div
      ref={ref}
      role="menu"
      aria-label="Add to playlist"
      style={style}
      className="glass z-50 w-56 rounded-card p-1 shadow-lg"
    >
      {playlists.length === 0 && (
        <p className="px-2 py-1.5 text-12 text-ink-3">No playlists yet.</p>
      )}
      {playlists.map((p) => (
        <button
          key={p.id}
          type="button"
          role="menuitem"
          onClick={() => onChoose(p.id)}
          className="block w-full truncate rounded-6 px-2 py-1.5 text-left text-13 text-ink-1 transition hover:bg-glass"
        >
          {p.name}
        </button>
      ))}
    </div>,
    document.body,
  );
}
