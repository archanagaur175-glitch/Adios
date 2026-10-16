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
  /** P9.7: iTunes hits carry their own id (relevance order from Apple). */
  itunesId?: number;
}

interface SearchState {
  open: boolean;
  query: string;
  localResults: Track[];
  onlineTracks: OnlineResult[];
  onlineAlbums: OnlineResult[];
  onlineArtists: OnlineResult[];
  /** Keyboard navigation index across the flattened grouped list. */
  activeIndex: number;
  onlinePending: boolean;

  /** Open the overlay, or close it (closing = full reset + leave #/search). */
  setOpen: (open: boolean) => void;
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
  open: false,
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

  /** P9.9: closing no longer touches location.hash — the /search page
      subscribes and navigates away on close (its old cleanup fought
      StrictMode and bounced the route; see routes/Search.tsx). */
  setOpen: (open) => set(open ? { open: true } : initial),
  // P9.8 (#3): mark online pending synchronously with the keystroke so the
  // first paint after typing can never flash "No matches" before the
  // debounce/effect runs (useEffect paints first, effects after).
  setQuery: (query) =>
    set(
      query.trim()
        ? { query, activeIndex: 0, onlinePending: true }
        : { query, activeIndex: 0 },
    ),
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
