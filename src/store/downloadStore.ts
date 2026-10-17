import { create } from "zustand";

/** P12: per-track download progress + on-disk badge set (pure zustand —
    no tauri imports, so it is unit-testable in node like playGate). */

export interface DownloadProgress {
  trackId: number;
  received: number;
  total: number | null;
}

interface DownloadState {
  /** trackId → progress of an active download. */
  active: Record<number, DownloadProgress>;
  /** trackIds confirmed on disk by the last batch check. */
  cachedIds: Record<number, true>;
  setProgress: (p: DownloadProgress) => void;
  clearActive: (trackId: number) => void;
  setCachedIds: (ids: number[]) => void;
  /** P12: merge ids into the badge set (single-download success). */
  addCachedIds: (ids: number[]) => void;
}

export const useDownloadStore = create<DownloadState>((set) => ({
  active: {},
  cachedIds: {},
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
}));
