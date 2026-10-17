import { create } from "zustand";

/** P12: per-track download progress + on-disk badge set (pure zustand —
    no tauri imports, so it is unit-testable in node like playGate). */

export interface DownloadProgress {
  trackId: number;
  received: number;
  total: number | null;
}

/** P14: a just-finished download, shown by the status panel until it fades. */
export interface DownloadDoneEntry {
  trackId: number;
  title: string;
}

interface DownloadState {
  /** trackId → progress of an active download. */
  active: Record<number, DownloadProgress>;
  /** trackIds confirmed on disk by the last batch check. */
  cachedIds: Record<number, true>;
  /** P14: recently completed downloads (status panel, newest last). */
  done: DownloadDoneEntry[];
  setProgress: (p: DownloadProgress) => void;
  clearActive: (trackId: number) => void;
  setCachedIds: (ids: number[]) => void;
  /** P12: merge ids into the badge set (single-download success). */
  addCachedIds: (ids: number[]) => void;
  /** P14: record a finished download (replaces an earlier entry for the id). */
  markDone: (entry: DownloadDoneEntry) => void;
  /** P14: drop a finished entry (after the panel's fade-out). */
  clearDone: (trackId: number) => void;
}

export const useDownloadStore = create<DownloadState>((set) => ({
  active: {},
  cachedIds: {},
  done: [],
  setProgress: (p) => set((s) => ({ active: { ...s.active, [p.trackId]: p } })),
  clearActive: (trackId) =>
    set((s) => {
      const active = { ...s.active };
      delete active[trackId];
      return { active };
    }),
  setCachedIds: (ids) =>
    set({ cachedIds: Object.fromEntries(ids.map((id) => [id, true as const])) }),
  addCachedIds: (ids) =>
    set((s) => ({
      cachedIds: {
        ...s.cachedIds,
        ...Object.fromEntries(ids.map((id) => [id, true as const])),
      },
    })),
  markDone: (entry) =>
    set((s) => ({
      done: [...s.done.filter((d) => d.trackId !== entry.trackId), entry],
    })),
  clearDone: (trackId) =>
    set((s) => ({ done: s.done.filter((d) => d.trackId !== trackId) })),
}));
