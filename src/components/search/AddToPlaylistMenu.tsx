/** P9.7: tiny playlist picker for "+ Add" on online search rows.
    Dropdown of playlist names; picking one persists the search hit as a
    'stream' library row (Rust add_stream_track) and links it (P8.2). */

import { useEffect, useRef, useState } from "react";
import { addPickToPlaylist, type OnlinePick } from "@/lib/onlinePlay";
import { listPlaylists } from "@/lib/playlistApi";
import type { PlaylistSummary } from "@/lib/types";

interface Props {
  pick: OnlinePick;
  onClose: () => void;
  /** Called after the row was added — the overlay flips it to "Added ✓". */
  onAdded: () => void;
}

export function AddToPlaylistMenu({ pick, onClose, onAdded }: Props) {
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

  return (
    <div
      ref={ref}
      role="menu"
      aria-label="Add to playlist"
      className="glass absolute right-0 top-full z-10 mt-1 w-56 rounded-card p-1"
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
    </div>
  );
}
