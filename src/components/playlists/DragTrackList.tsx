import { useEffect, useRef, useState } from "react";
import { formatDuration } from "@/lib/format";
import type { Track } from "@/lib/types";

/** Uniform row height (px) — drag math assumes fixed-height rows. */
const ROW_H = 48;

interface Props {
  tracks: Track[];
  /** Persist the new full order (track ids, top→bottom). */
  onReorder: (trackIds: number[]) => void;
  onRemove: (trackId: number) => void;
  onPlay: (trackId: number) => void;
}

function move<T>(items: T[], from: number, to: number): T[] {
  const next = items.slice();
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

/** Pointer-drag reorder list (P8.2): window listeners (no pointer capture,
    so synthetic CDP events work too); the order commits on pointerup. */
export function DragTrackList({ tracks, onReorder, onRemove, onPlay }: Props) {
  const [order, setOrder] = useState<Track[]>(tracks);
  const [drag, setDrag] = useState<
    { from: number; startY: number; snapshot: Track[] } | null
  >(null);
  const latestOrder = useRef<Track[]>(tracks);

  // Props win whenever we are not mid-drag (reload/removal from outside).
  useEffect(() => {
    if (!drag) setOrder(tracks);
  }, [tracks, drag]);

  useEffect(() => {
    if (!drag) return;
    const orig = drag.snapshot;
    latestOrder.current = orig;
    const onMove = (e: PointerEvent) => {
      const delta = e.clientY - drag.startY;
      const to = Math.min(
        orig.length - 1,
        Math.max(0, drag.from + Math.round(delta / ROW_H)),
      );
      const next = move(orig, drag.from, to);
      latestOrder.current = next;
      setOrder(next);
    };
    const onUp = () => {
      setDrag(null);
      onReorder(latestOrder.current.map((t) => t.id));
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
  }, [drag, onReorder]);

  return (
    <ul className="flex flex-col gap-1" data-testid="playlist-tracks">
      {order.map((track, index) => (
        <li
          key={track.id}
          className={`grid h-12 cursor-grab grid-cols-[auto_1fr_auto_auto] items-center gap-3 rounded-card px-3 transition hover:bg-glass ${
            drag?.snapshot[drag.from]?.id === track.id ? "bg-glass opacity-60" : ""
          }`}
          data-track-id={track.id}
        >
          <span
            aria-label="Reorder"
            title="Drag to reorder"
            onPointerDown={(e) => {
              e.preventDefault();
              setDrag({ from: index, startY: e.clientY, snapshot: order });
            }}
            className="cursor-grab select-none text-ink-3 active:cursor-grabbing"
          >
            ⠿
          </span>
          <button
            type="button"
            onClick={() => onPlay(track.id)}
            className="min-w-0 truncate text-left text-14 text-ink-1"
          >
            {track.artist} — {track.title}
          </button>
          <span className="text-12 tabular text-ink-3">
            {formatDuration(track.durationMs)}
          </span>
          <button
            type="button"
            aria-label={`Remove ${track.title}`}
            onClick={() => onRemove(track.id)}
            className="rounded px-2 py-1 text-14 text-ink-3 transition hover:text-danger"
          >
            ×
          </button>
        </li>
      ))}
    </ul>
  );
}
