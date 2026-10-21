import { useEffect, useRef } from "react";
import type { LrcLine } from "@/lib/lrc";

/** Synced lyric lines list (DESIGN §7): active line scale 1.06, inactive
    blur up to 3 px by distance, click = seek, 400 ms eased scroll that
    writes scrollTop directly via rAF (never through React state). */

type Props = {
  lines: LrcLine[];
  activeIndex: number;
  onSeek: (timeMs: number) => void;
  className?: string;
};

const SCROLL_MS = 400;

function easeInOutCubic(p: number): number {
  return p < 0.5 ? 4 * p * p * p : 1 - Math.pow(-2 * p + 2, 3) / 2;
}

export function LyricsLines({ lines, activeIndex, onSeek, className }: Props) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const activeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const container = scrollRef.current;
    const el = activeRef.current;
    if (!container || !el || activeIndex < 0) return;
    const max = container.scrollHeight - container.clientHeight;
    if (max <= 0) return;
    // Rect math: el.offsetTop is measured against the nearest positioned
    // ancestor (the fixed .np), NOT this container — computing from rects
    // keeps the top-align correct wherever .ly lives (panel, immersive, phone).
    const cRect = container.getBoundingClientRect();
    const eRect = el.getBoundingClientRect();
    const elTop = eRect.top - cRect.top + container.scrollTop;
    // Top-align (user decision 2026-10-20): pin the active line to the top
    // of the box so it is visible by default, upcoming lines read below it.
    const to = Math.max(0, Math.min(max, elTop));
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      container.scrollTop = to;
      return;
    }
    const from = container.scrollTop;
    const t0 = performance.now();
    let raf = requestAnimationFrame(function step(t) {
      const p = Math.min(1, (t - t0) / SCROLL_MS);
      container.scrollTop = from + (to - from) * easeInOutCubic(p);
      if (p < 1) raf = requestAnimationFrame(step);
    });
    return () => cancelAnimationFrame(raf);
  }, [activeIndex]);

  return (
    <div className={`ly ${className ?? ""}`.trim()} ref={scrollRef}>
      {lines.map((line, i) => {
        const active = i === activeIndex;
        const dist = Math.abs(i - activeIndex);
        return (
          <button
            key={`${line.timeMs}-${i}`}
            type="button"
            ref={active ? activeRef : undefined}
            className={`ly-line${active ? " ly-line--active" : ""}`}
            style={active ? undefined : { filter: `blur(${Math.min(3, dist)}px)` }}
            aria-current={active ? "true" : undefined}
            onClick={() => onSeek(line.timeMs)}
          >
            {line.text || "\u00A0"}
          </button>
        );
      })}
    </div>
  );
}
