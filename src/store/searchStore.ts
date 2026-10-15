import { create } from "zustand";
import type { Track } from "@/lib/types";

/** Search overlay state. Local (FTS5) results arrive sync,
 *  online (MusicBrainz) results append after the 250–300 ms debounce (D12). */

export interface OnlineResult {
  mbid: string;
  title: string;
  artist: string;
  album?: string;
  durationMs?: number;
}

interface SearchState {
  query: string;
  localResults: Track[];
  onlineTracks: OnlineResult[];
  onlineAlbums: OnlineResult[];
  onlineArtists: OnlineResult[];
  /** Keyboard navigation index across the flattened grouped list. */
  activeIndex: number;
  onlinePending: boolean;

  setQuery: (query: string) => void;
  setLocalResults: (results: Track[]) => void;
  setOnlineResults: (results: {
    tracks?: OnlineResult[];
    albums?: OnlineResult[];
    artists?: OnlineResult[];
  }) => void;
  setActiveIndex: (index: number) => void;
  setOnlinePending: (pending: boolean) => void;
  reset: () => void;
}

const initial = {
  query: "",
  localResults: [] as Track[],
  onlineTracks: [] as OnlineResult[],
  onlineAlbums: [] as OnlineResult[],
  onlineArtists: [] as OnlineResult[],
  activeIndex: 0,
  onlinePending: false,
};

export const useSearchStore = create<SearchState>((set) => ({
  ...initial,

  setQuery: (query) => set({ query, activeIndex: 0 }),
  setLocalResults: (localResults) => set({ localResults }),
  setOnlineResults: (results) =>
    set((state) => ({
      onlineTracks: results.tracks ?? state.onlineTracks,
      onlineAlbums: results.albums ?? state.onlineAlbums,
      onlineArtists: results.artists ?? state.onlineArtists,
      onlinePending: false,
    })),
  setActiveIndex: (activeIndex) => set({ activeIndex }),
  setOnlinePending: (onlinePending) => set({ onlinePending }),
  reset: () => set(initial),
}));
