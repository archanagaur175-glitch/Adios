import { usePlayerStore } from "@/store/playerStore";
import { LyricsLines } from "./LyricsLines";
import { useLyrics } from "./useLyrics";
import "./lyrics.css";

/** Lyrics content for the current track: synced lines (DESIGN §7), plain
    paragraphs, instrumental / none / offline notes. Used by the Now Playing
    panel and the immersive overlay (via className for the large variant). */

type Props = { className?: string };

export function LyricsPanel({ className }: Props) {
  const trackId = usePlayerStore((s) => s.currentTrackId);
  const st = useLyrics(trackId);

  if (st.status === "loading") {
    return <p className="ly__note">Loading…</p>;
  }
  if (st.status === "unavailable") {
    return <p className="ly__note">Lyrics unavailable — offline</p>;
  }
  if (st.lines.length > 0) {
    return (
      <LyricsLines
        lines={st.lines}
        activeIndex={st.activeIndex}
        className={className}
        onSeek={(ms) => usePlayerStore.getState().seekTo(ms / 1000)}
      />
    );
  }
  if (st.instrumental) {
    return <p className="ly__note">Instrumental — no lyrics.</p>;
  }
  if (st.plain) {
    return (
      <div className={`ly__plain ${className ?? ""}`.trim()}>
        {st.plain.split(/\r?\n/).map((para, i) => (
          <p key={i}>{para || "\u00A0"}</p>
        ))}
      </div>
    );
  }
  if (!st.found) {
    return <p className="ly__note">No lyrics found.</p>;
  }
  return <p className="ly__note">No lyrics available.</p>;
}
