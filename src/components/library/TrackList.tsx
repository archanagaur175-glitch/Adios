import type { ReactNode } from "react";
import { formatDuration } from "@/lib/format";
import type { Track } from "@/lib/types";

/** P10: shared track rows for the Library views — title line + optional
    album, right slot for duration/count, click or Enter plays from the list. */
export function TrackList({
  tracks,
  onPlay,
  right,
}: {
  tracks: Track[];
  onPlay: (trackId: number) => void;
  /** Rendered in the right column (defaults to duration). */
  right?: (track: Track) => ReactNode;
}) {
  return (
    <ul className="flex flex-col gap-1 overflow-y-auto">
      {tracks.map((track) => (
        <li
          key={track.id}
          onClick={() => onPlay(track.id)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onPlay(track.id);
          }}
          tabIndex={0}
          role="button"
          className="grid cursor-pointer grid-cols-[1fr_auto] items-center gap-3 rounded-card px-3 py-2 transition hover:bg-glass focus-visible:bg-glass"
        >
          <div className="min-w-0">
            <p className="truncate text-14 text-ink-1">
              {track.artist} — {track.title}
            </p>
            {track.album && (
              <p className="truncate text-12 text-ink-3">{track.album}</p>
            )}
          </div>
          <span className="text-12 tabular text-ink-3">
            {right ? right(track) : formatDuration(track.durationMs)}
          </span>
        </li>
      ))}
    </ul>
  );
}
