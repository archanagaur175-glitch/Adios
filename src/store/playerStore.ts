import { create } from "zustand";

/** Runtime playback state. Engine (src/audio) subscribes to this store. */

export type RepeatMode = "off" | "queue" | "track";

interface PlayerState {
  /** Track ids in play order. */
  queue: number[];
  queueIndex: number;
  currentTrackId: number | null;
  isPlaying: boolean;
  /** Seconds. */
  positionSec: number;
  durationSec: number;
  /** 0..1 */
  volume: number;
  muted: boolean;
  /** 0–8000 ms; 0 = preload-and-switch (default, D3). */
  crossfadeMs: number;
  repeatMode: RepeatMode;
  shuffle: boolean;

  setQueue: (trackIds: number[], startIndex?: number) => void;
  setPlaying: (playing: boolean) => void;
  setPosition: (sec: number) => void;
  setDuration: (sec: number) => void;
  setVolume: (volume: number) => void;
  setMuted: (muted: boolean) => void;
  setCrossfadeMs: (ms: number) => void;
  setRepeatMode: (mode: RepeatMode) => void;
  setShuffle: (shuffle: boolean) => void;
  next: () => void;
  prev: () => void;
}

const clamp = (value: number, min: number, max: number) =>
  Math.min(max, Math.max(min, value));

export const usePlayerStore = create<PlayerState>((set, get) => ({
  queue: [],
  queueIndex: -1,
  currentTrackId: null,
  isPlaying: false,
  positionSec: 0,
  durationSec: 0,
  volume: 0.8,
  muted: false,
  crossfadeMs: 0,
  repeatMode: "off",
  shuffle: false,

  setQueue: (trackIds, startIndex = 0) =>
    set({
      queue: trackIds,
      queueIndex: clamp(startIndex, 0, trackIds.length - 1),
      currentTrackId: trackIds[startIndex] ?? null,
    }),
  setPlaying: (isPlaying) => set({ isPlaying }),
  setPosition: (positionSec) => set({ positionSec }),
  setDuration: (durationSec) => set({ durationSec }),
  setVolume: (volume) => set({ volume: clamp(volume, 0, 1), muted: false }),
  setMuted: (muted) => set({ muted }),
  setCrossfadeMs: (ms) => set({ crossfadeMs: clamp(Math.round(ms), 0, 8000) }),
  setRepeatMode: (repeatMode) => set({ repeatMode }),
  setShuffle: (shuffle) => set({ shuffle }),

  next: () => {
    const { queue, queueIndex, repeatMode } = get();
    if (queue.length === 0) return;
    let index = queueIndex + 1;
    if (index >= queue.length) {
      if (repeatMode === "queue") index = 0;
      else return;
    }
    set({ queueIndex: index, currentTrackId: queue[index] ?? null });
  },
  prev: () => {
    const { queue, queueIndex } = get();
    if (queue.length === 0) return;
    const index = Math.max(0, queueIndex - 1);
    set({ queueIndex: index, currentTrackId: queue[index] ?? null });
  },
}));
