import { create } from "zustand";
import { persist } from "zustand/middleware";

/** Persisted settings mirror (source of truth: SQLite `settings` table from P2).
 *  Interim persistence: localStorage until the Rust settings commands land (P2.2),
 *  then the same shape is hydrated/saved through invoke. */

export interface SettingsState {
  /** MusicBrainz contact email (D2); empty = placeholder UA + one-time warning. */
  contactEmail: string;
  /** Resolver chain order; names of resolver modules. */
  resolverOrder: string[];
  /** Piped/Invidious instance URLs; EMPTY by default (D4). */
  pipedInstances: string[];
  /** Disk cache cap for remote streams. */
  cacheSizeMb: number;
  /** Visualizer quality tier default (uiStore holds the active one). */
  defaultTier: "low" | "med" | "high";
  /** 0–8000 ms; 0 = off (D3). */
  crossfadeMs: number;
  /** True once the "set your contact email" warning has been shown. */
  mbWarningShown: boolean;

  set: (patch: Partial<SettingsState>) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      contactEmail: "",
      resolverOrder: ["local", "cache", "ytdlp", "piped"],
      pipedInstances: [],
      cacheSizeMb: 512,
      defaultTier: "med",
      crossfadeMs: 0,
      mbWarningShown: false,

      set: (patch) => set(patch),
    }),
    { name: "adios-settings" },
  ),
);
