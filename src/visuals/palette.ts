/** P6.1 palette wiring: the P5.2 track-row palette (accent / accent2 /
    dominant_colors_json — already extracted by Rust k-means) republished as
    `--palette-*` CSS vars on `:root`. This is the feed the 6.2 aurora and
    6.4 hero read — no extraction happens in JS, only lookup + publish. */

import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { onAccentFor } from "@/lib/color";
import type { Track } from "@/lib/types";

const ROOT = () => document.documentElement.style;

function clearVars(): void {
  const root = ROOT();
  for (const name of [
    "--palette-accent",
    "--palette-accent2",
    "--palette-1",
    "--palette-2",
    "--palette-3",
    "--on-accent",
  ]) {
    root.removeProperty(name);
  }
}

/** Publish one track's palette (null/missing colors → vars cleared). */
export function applyTrackPalette(track: Track | null | undefined): void {
  const root = ROOT();
  if (!track) {
    clearVars();
    return;
  }
  const colors: (string | null | undefined)[] = (() => {
    try {
      const parsed: unknown = track.dominantColorsJson
        ? JSON.parse(track.dominantColorsJson)
        : null;
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  })();

  if (track.accent) root.setProperty("--palette-accent", track.accent);
  else root.removeProperty("--palette-accent");
  // OQ4: readable button/chip text for this accent (falls back to the
  // token default when the track has no accent).
  root.setProperty("--on-accent", onAccentFor(track.accent));
  if (track.accent2) root.setProperty("--palette-accent2", track.accent2);
  else root.removeProperty("--palette-accent2");
  colors.forEach((color, i) => {
    if (typeof color === "string" && i < 3) root.setProperty(`--palette-${i + 1}`, color);
  });
  for (let i = colors.length; i < 3; i++) root.removeProperty(`--palette-${i + 1}`);
}

function current(): Track | null {
  const { currentTrackId } = usePlayerStore.getState();
  if (currentTrackId == null) return null;
  return useLibraryStore.getState().tracks.find((t) => t.id === currentTrackId) ?? null;
}

/** Republish on track change and whenever the library rows reload
    (scan/enrich refresh `tracks`, which can newly carry palette data).
    Returns cleanup. */
export function initPaletteWiring(): () => void {
  let lastId = usePlayerStore.getState().currentTrackId;
  let lastRows = useLibraryStore.getState().tracks;
  const unsub = usePlayerStore.subscribe((s) => {
    if (s.currentTrackId !== lastId) {
      lastId = s.currentTrackId;
      applyTrackPalette(current());
    }
  });
  const unsubLib = useLibraryStore.subscribe((s) => {
    if (s.tracks !== lastRows) {
      lastRows = s.tracks;
      applyTrackPalette(current());
    }
  });
  applyTrackPalette(current());
  return () => {
    unsub();
    unsubLib();
    clearVars();
  };
}
