import { useEffect, useRef, useState } from "react";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";

/** P9.1 polite live region: announces track changes and play/pause only —
    NEVER position updates (timeupdate would flood screen readers). Boot
    session-restore (queue set while paused) is suppressed until the first
    real play so launches stay silent. */
export function Announcer() {
  const currentTrackId = usePlayerStore((s) => s.currentTrackId);
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const track = useLibraryStore(
    (s) => s.tracks.find((t) => t.id === currentTrackId) ?? null,
  );
  const [message, setMessage] = useState("");
  const startedRef = useRef(false);
  const prevRef = useRef({ trackId: currentTrackId, isPlaying });

  useEffect(() => {
    const prev = prevRef.current;
    prevRef.current = { trackId: currentTrackId, isPlaying };
    if (isPlaying) startedRef.current = true;
    if (!startedRef.current) return;
    const label = track ? `${track.title} by ${track.artist}` : "";
    if (currentTrackId !== prev.trackId && isPlaying) {
      setMessage(label ? `Playing ${label}` : "Playing");
    } else if (isPlaying !== prev.isPlaying) {
      setMessage(isPlaying ? (label ? `Playing ${label}` : "Playing") : "Paused");
    }
  }, [currentTrackId, isPlaying, track]);

  return (
    <div role="status" aria-live="polite" className="sr-only">
      {message}
    </div>
  );
}
