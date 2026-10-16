import { useEffect, useMemo, useState } from "react";
import { getLyrics } from "@/lib/api";
import { activeLineIndex, parseLrc, type LrcLine } from "@/lib/lrc";
import type { LyricsPayload } from "@/lib/types";
import { usePlayerStore } from "@/store/playerStore";

/** Resolved lyrics for one track (P7.2). Fetches ONCE per trackId — 404
    negatives are cached server-side, errors (offline) never auto-retry. */

export interface LyricsState {
  status: "loading" | "ready" | "unavailable";
  /** Parsed synced lines (empty for plain-only / none). */
  lines: LrcLine[];
  plain: string | null;
  instrumental: boolean;
  /** False when LRCLIB had nothing (negative cache row). */
  found: boolean;
  /** Last line at or before the playhead; -1 before the first. */
  activeIndex: number;
  /** Raw payload once resolved (null while loading / on error). */
  payload: LyricsPayload | null;
}

export function useLyrics(trackId: number | null): LyricsState {
  const [payload, setPayload] = useState<LyricsPayload | null>(null);
  const [failed, setFailed] = useState(false);
  const positionMs = usePlayerStore((s) => s.positionSec) * 1000;

  useEffect(() => {
    if (trackId == null) {
      setPayload(null);
      setFailed(false);
      return;
    }
    let cancelled = false;
    setPayload(null);
    setFailed(false);
    getLyrics(trackId)
      .then((p) => {
        if (!cancelled) setPayload(p);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [trackId]);

  const lines = useMemo(
    () => (payload?.synced ? parseLrc(payload.synced) : []),
    [payload?.synced],
  );
  const activeIndex = useMemo(
    () => (lines.length > 0 ? activeLineIndex(lines, positionMs) : -1),
    [lines, positionMs],
  );

  if (failed) {
    return { status: "unavailable", lines: [], plain: null, instrumental: false, found: false, activeIndex: -1, payload: null };
  }
  if (!payload) {
    return { status: "loading", lines: [], plain: null, instrumental: false, found: false, activeIndex: -1, payload: null };
  }
  return {
    status: "ready",
    lines,
    plain: payload.plain ?? null,
    instrumental: payload.instrumental,
    found: payload.found,
    activeIndex,
    payload,
  };
}
