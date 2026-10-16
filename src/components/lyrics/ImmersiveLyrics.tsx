import { useEffect } from "react";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";
import { LyricsPanel } from "./LyricsPanel";
import "./lyrics.css";

/** Full-screen immersive lyrics (DESIGN §7: "full-screen large type, chrome
    hidden"). Toggled from the player-bar lyrics button or the L key; ESC or
    × closes. Mounts app-wide; renders null while closed (fetch is gated on
    open, so a closed overlay never calls LRCLIB). */

export function ImmersiveLyrics() {
  const open = useUiStore((s) => s.lyricsOpen);
  const currentTrackId = usePlayerStore((s) => s.currentTrackId);
  const track = useLibraryStore((s) =>
    s.tracks.find((t) => t.id === currentTrackId) ?? null,
  );

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") useUiStore.getState().setLyricsOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  if (!open) return null;

  return (
    <div className="ly-imm" role="dialog" aria-label="Lyrics">
      <header className="ly-imm__bar">
        <p className="ly-imm__title">
          {track ? `${track.title} — ${track.artist}` : "Nothing playing"}
        </p>
        <button
          type="button"
          className="ly-imm__close"
          aria-label="Close lyrics"
          onClick={() => useUiStore.getState().setLyricsOpen(false)}
        >
          ×
        </button>
      </header>
      <div className="ly-imm__body">
        <LyricsPanel className="ly--imm" />
      </div>
    </div>
  );
}
