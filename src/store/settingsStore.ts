import { create } from "zustand";
import { getSettings, setSetting } from "@/lib/api";

/** Persisted settings mirror (source of truth: SQLite `settings` table).
 *  Hydrated from Rust at startup; every `set` writes through via invoke. */

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

  /** Load persisted settings from SQLite (call once at startup). */
  hydrate: () => Promise<void>;
  /** Patch settings into the store and write each key through to SQLite. */
  set: (patch: Partial<SettingsState>) => void;
}

const defaults: Omit<SettingsState, "hydrate" | "set"> = {
  contactEmail: "",
  resolverOrder: ["local", "cache", "ytdlp", "piped"],
  pipedInstances: [],
  cacheSizeMb: 512,
  defaultTier: "med",
  crossfadeMs: 0,
  mbWarningShown: false,
};

export const useSettingsStore = create<SettingsState>((set) => ({
  ...defaults,

  hydrate: async () => {
    const stored = await getSettings();
    const known = Object.keys(defaults) as (keyof typeof defaults)[];
    const patch: Partial<SettingsState> = {};
    for (const key of known) {
      if (key in stored) {
        (patch as Record<string, unknown>)[key] = stored[key];
      }
    }
    set(patch);
  },

  set: (patch) => {
    set(patch);
    for (const [key, value] of Object.entries(patch)) {
      void setSetting(key, value);
    }
  },
}));
