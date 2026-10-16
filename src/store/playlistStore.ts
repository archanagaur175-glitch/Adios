import { create } from "zustand";
import { listPlaylists } from "@/lib/playlistApi";
import type { PlaylistSummary } from "@/lib/types";

/** Playlist summaries cache (P8.2); detail pages fetch their own rows. */

export interface PlaylistStoreState {
  playlists: PlaylistSummary[];
  /** Re-read the summary list from SQLite. */
  load: () => Promise<void>;
}

export const usePlaylistStore = create<PlaylistStoreState>((set) => ({
  playlists: [],

  load: async () => {
    set({ playlists: await listPlaylists() });
  },
}));
