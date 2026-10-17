import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { QualityTier } from "@/lib/types";

/** Transient UI state + a few persisted preferences. */

export interface Toast {
  id: number;
  message: string;
  tone: "info" | "danger";
}

interface UiState {
  sidebarCollapsed: boolean;
  nowPlayingOpen: boolean;
  lyricsOpen: boolean;
  queueOpen: boolean;
  /** Visualizer quality tier; Eco is a boolean so battery auto-suggest stays separate. */
  qualityTier: QualityTier;
  ecoMode: boolean;
  toasts: Toast[];
  /** P15: per-playlist "recommendations" toggle (playlistId → on). */
  recPrefs: Record<number, boolean>;

  setSidebarCollapsed: (collapsed: boolean) => void;
  setNowPlayingOpen: (open: boolean) => void;
  setLyricsOpen: (open: boolean) => void;
  setQueueOpen: (open: boolean) => void;
  setQualityTier: (tier: QualityTier) => void;
  setEcoMode: (eco: boolean) => void;
  pushToast: (toast: Omit<Toast, "id">) => void;
  dismissToast: (id: number) => void;
  setRecPref: (playlistId: number, on: boolean) => void;
}

let nextToastId = 1;

export const useUiStore = create<UiState>()(
  persist(
    (set, get) => ({
      sidebarCollapsed: false,
      nowPlayingOpen: false,
      lyricsOpen: false,
      queueOpen: false,
      qualityTier: "med",
      ecoMode: false,
      toasts: [],
      recPrefs: {},

      setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
      setNowPlayingOpen: (nowPlayingOpen) => set({ nowPlayingOpen }),
      setLyricsOpen: (lyricsOpen) => set({ lyricsOpen }),
      setQueueOpen: (queueOpen) => set({ queueOpen }),
      setQualityTier: (qualityTier) => set({ qualityTier }),
      setEcoMode: (ecoMode) => set({ ecoMode }),
      pushToast: (toast) =>
        set({ toasts: [...get().toasts, { ...toast, id: nextToastId++ }] }),
      dismissToast: (id) =>
        set({ toasts: get().toasts.filter((t) => t.id !== id) }),
      setRecPref: (playlistId, on) =>
        set({ recPrefs: { ...get().recPrefs, [playlistId]: on } }),
    }),
    {
      name: "adios-ui",
      partialize: (state) => ({
        sidebarCollapsed: state.sidebarCollapsed,
        qualityTier: state.qualityTier,
        recPrefs: state.recPrefs,
      }),
    },
  ),
);
