import { formatDuration } from "@/lib/format";
import { CoverArt } from "@/components/common/CoverArt";
import { usePlayerStore } from "@/store/playerStore";
import type { Track } from "@/lib/types";

/** Shared Home shelf (P16): header with a Play action over a clickable
    track list. Renders nothing when empty — shelves self-gate. */

interface Props {
  title: string;
  /** Extra header text ("Because you listened to" → the artist name). */
  subtitle?: string;
  tracks: Track[];
}

export function TrackShelf({ title, subtitle, tracks }: Props) {
  if (tracks.length === 0) return null;
  const ids = tracks.map((t) => t.id);
  const label = subtitle ? `${title} — ${subtitle}` : title;
  const playFrom = (index: number) => usePlayerStore.getState().playQueue(ids, index);
  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between gap-3">
        <h2 className="text-11 font-medium uppercase tracking-wide text-ink-3">
          {title}
          {subtitle && <span className="ml-2 normal-case text-ink-2">— {subtitle}</span>}
        </h2>
        <button
          type="button"
          onClick={() => playFrom(0)}
          aria-label={`Play ${label}`}
          className="rounded-ctrl border border-stroke px-3 py-1 text-13 text-ink-2 transition hover:bg-glass"
        >
          Play
        </button>
      </div>
      <ul className="flex flex-col gap-1">
        {tracks.map((track, index) => (
          <li key={track.id}>
            <button
              type="button"
              onClick={() => playFrom(index)}
              className="grid w-full grid-cols-[auto_1fr_auto] items-center gap-3 rounded-card px-3 py-2 text-left transition hover:bg-glass focus-visible:bg-glass"
            >
              <CoverArt track={track} size={36} />
              <span className="min-w-0">
                <span className="block truncate text-14 text-ink-1">
                  {track.artist} — {track.title}
                </span>
                {track.album && (
                  <span className="block truncate text-12 text-ink-3">{track.album}</span>
                )}
              </span>
              <span className="text-12 tabular text-ink-3">{formatDuration(track.durationMs)}</span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
