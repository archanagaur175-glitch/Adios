/** P9.7/P9.8: playlist picker for "+ Add" on online search rows. P9.8 #1:
    the menu used to render inside the search dialog — which has
    overflow-hidden — so it opened CLIPPED OUT OF VIEW (click looked dead).
    It now portals to document.body with fixed coordinates anchored to the
    clicked button, flipping upward near the viewport bottom. */

import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { addPickToPlaylist, type OnlinePick } from "@/lib/onlinePlay";
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
  pick: OnlinePick;
  anchor: MenuAnchor;
  onClose: () => void;
  /** Called after the row was added — the overlay flips it to "Added ✓". */
  onAdded: () => void;
}

export function AddToPlaylistMenu({ pick, anchor, onClose, onAdded }: Props) {
  const [playlists, setPlaylists] = useState<PlaylistSummary[]>([]);
  const [busy, setBusy] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

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

  async function choose(playlistId: number): Promise<void> {
    if (busy) return;
    setBusy(true);
    try {
      const ok = await addPickToPlaylist(pick, playlistId);
      if (ok) onAdded();
      else onClose();
    } finally {
      setBusy(false);
    }
  }

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
          disabled={busy}
          onClick={() => void choose(p.id)}
          className="block w-full truncate rounded-6 px-2 py-1.5 text-left text-13 text-ink-1 transition hover:bg-glass disabled:opacity-50"
        >
          {busy ? "Adding…" : p.name}
        </button>
      ))}
    </div>,
    document.body,
  );
}
