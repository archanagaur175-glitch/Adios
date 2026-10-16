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
  /** P9.5: transient online track (MusicBrainz pick → resolve_query). Lives
      in-memory only (negative id, never persisted; session restore drops it).
      Dedupe by mbRecordingId — re-picking refreshes the stream URL. */
  upsertStreamTrack: (
    input: Omit<Track, "id" | "playCount"> & { mbRecordingId: string },
  ) => number;
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
  upsertStreamTrack: (input) => {
    let id = 0;
    set((state) => {
      const existing = state.tracks.find(
        (t) => t.mbRecordingId === input.mbRecordingId,
      );
      if (existing) {
        id = existing.id;
        return {
          tracks: state.tracks.map((t) =>
            t.id === existing.id ? { ...t, ...input, id: existing.id } : t,
          ),
        };
      }
      id = Math.min(0, ...state.tracks.map((t) => t.id)) - 1;
      return { tracks: [...state.tracks, { ...input, id, playCount: 0 }] };
    });
    return id;
  },
}));
