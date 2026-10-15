import { create } from "zustand";
import type { Playlist, Track } from "@/lib/types";

/** Library content: tracks, playlists, scan progress, filters. */

interface ScanProgress {
  scanned: number;
  total: number;
  folder: string;
}

interface LibraryState {
  tracks: Track[];
  playlists: Playlist[];
  /** playlist id -> ordered track ids */
  playlistTracks: Record<number, number[]>;
  filters: { artist?: string; album?: string };
  scanProgress: ScanProgress | null;
  hydrated: boolean;

  setTracks: (tracks: Track[]) => void;
  setPlaylists: (playlists: Playlist[]) => void;
  setPlaylistTracks: (playlistId: number, trackIds: number[]) => void;
  setFilters: (filters: LibraryState["filters"]) => void;
  setScanProgress: (progress: ScanProgress | null) => void;
  setHydrated: (hydrated: boolean) => void;
}

export const useLibraryStore = create<LibraryState>((set) => ({
  tracks: [],
  playlists: [],
  playlistTracks: {},
  filters: {},
  scanProgress: null,
  hydrated: false,

  setTracks: (tracks) => set({ tracks }),
  setPlaylists: (playlists) => set({ playlists }),
  setPlaylistTracks: (playlistId, trackIds) =>
    set((state) => ({
      playlistTracks: { ...state.playlistTracks, [playlistId]: trackIds },
    })),
  setFilters: (filters) => set({ filters }),
  setScanProgress: (scanProgress) => set({ scanProgress }),
  setHydrated: (hydrated) => set({ hydrated }),
}));
