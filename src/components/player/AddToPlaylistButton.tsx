import { useRef, useState } from "react";
import {
  AddToPlaylistMenu,
  type MenuAnchor,
} from "@/components/search/AddToPlaylistMenu";
import { addPlaylistTrack } from "@/lib/playlistApi";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";
import { IconAddToList } from "./icons";

/** P15.3: player-bar "Add to playlist" — links the CURRENT track's library
    row into a chosen playlist (no resolve needed; the row already exists).
    Menu portals to document.body and flips above the bar. */

export function AddToPlaylistButton() {
  const currentTrackId = usePlayerStore((s) => s.currentTrackId);
  const [anchor, setAnchor] = useState<MenuAnchor | null>(null);
  const btnRef = useRef<HTMLButtonElement>(null);

  function open() {
    const r = btnRef.current?.getBoundingClientRect();
    if (!r) return;
    setAnchor({ left: r.left, top: r.top, bottom: r.bottom, right: r.right });
  }

  async function choose(playlistId: number) {
    setAnchor(null);
    if (currentTrackId == null) return;
    const title =
      useLibraryStore.getState().tracks.find((t) => t.id === currentTrackId)
        ?.title ?? "";
    const pushToast = useUiStore.getState().pushToast;
    try {
      const ok = await addPlaylistTrack(playlistId, currentTrackId);
      pushToast({
        message: ok ? `Added “${title}” to playlist` : "Already in this playlist",
        tone: "info",
      });
    } catch (err) {
      console.error("[playerBar] add to playlist failed:", err);
      pushToast({ message: `Could not add “${title}” — ${String(err)}`, tone: "danger" });
    }
  }

  return (
    <>
      <button
        ref={btnRef}
        type="button"
        aria-label="Add to playlist"
        aria-expanded={anchor != null}
        disabled={currentTrackId == null}
        onClick={open}
        className="player-bar__btn"
      >
        <IconAddToList />
      </button>
      {anchor && (
        <AddToPlaylistMenu
          anchor={anchor}
          onClose={() => setAnchor(null)}
          onChoose={(playlistId) => void choose(playlistId)}
        />
      )}
    </>
  );
}
