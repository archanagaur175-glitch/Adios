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
  /** Bumped by seekTo; the engine listens and applies it (no echo loop). */
  seekVersion: number;
  /** Bumped by playQueue; play-history counts one play per epoch or track
      change (P10) so replaying the same track still increments the count. */
  playEpoch: number;

  setQueue: (trackIds: number[], startIndex?: number) => void;
  /** Atomically set queue + current track + start playback. */
  playQueue: (trackIds: number[], startIndex?: number) => void;
  setPlaying: (playing: boolean) => void;
  setPosition: (sec: number) => void;
  setDuration: (sec: number) => void;
  /** Request an engine seek; bumps seekVersion. */
  seekTo: (sec: number) => void;
  setVolume: (volume: number) => void;
  setMuted: (muted: boolean) => void;
  setCrossfadeMs: (ms: number) => void;
  setRepeatMode: (mode: RepeatMode) => void;
  setShuffle: (shuffle: boolean) => void;
  /** P15: insert a rec at a queue position; keeps the current track playing
      (shifts queueIndex when inserting at/before it). */
  insertQueueAt: (trackId: number, index: number) => void;
  /** P15: drop queue entries by id, never the current track; recomputes
      queueIndex from the kept entries. */
  removeQueueTracks: (ids: number[], keepId: number | null) => void;
  next: () => void;
  prev: () => void;
  /** Id the queue would advance to (for preloading); null at queue end. */
  peekNextId: () => number | null;
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
  seekVersion: 0,
  playEpoch: 0,

  setQueue: (trackIds, startIndex = 0) =>
    set({
      queue: trackIds,
      queueIndex: clamp(startIndex, 0, trackIds.length - 1),
      currentTrackId: trackIds[startIndex] ?? null,
    }),
  playQueue: (trackIds, startIndex = 0) =>
    set((state) => ({
      queue: trackIds,
      queueIndex: clamp(startIndex, 0, trackIds.length - 1),
      currentTrackId: trackIds[startIndex] ?? null,
      isPlaying: true,
      positionSec: 0,
      durationSec: 0,
      playEpoch: state.playEpoch + 1,
    })),
  setPlaying: (isPlaying) => set({ isPlaying }),
  setPosition: (positionSec) => set({ positionSec }),
  setDuration: (durationSec) => set({ durationSec }),
  seekTo: (sec) => {
    const { durationSec } = get();
    const clamped = clamp(sec, 0, durationSec > 0 ? durationSec : Math.max(sec, 0));
    set((state) => ({
      positionSec: clamped,
      seekVersion: state.seekVersion + 1,
    }));
  },
  setVolume: (volume) => set({ volume: clamp(volume, 0, 1), muted: false }),
  setMuted: (muted) => set({ muted }),
  setCrossfadeMs: (ms) => set({ crossfadeMs: clamp(Math.round(ms), 0, 8000) }),
  setRepeatMode: (repeatMode) => set({ repeatMode }),
  setShuffle: (shuffle) => set({ shuffle }),

  insertQueueAt: (trackId, index) =>
    set((state) => {
      const at = clamp(Math.round(index), 0, state.queue.length);
      const queue = [...state.queue];
      queue.splice(at, 0, trackId);
      const queueIndex =
        at <= state.queueIndex && state.queueIndex >= 0
          ? state.queueIndex + 1
          : state.queueIndex;
      return { queue, queueIndex };
    }),
  removeQueueTracks: (ids, keepId) =>
    set((state) => {
      if (state.queue.length === 0) return state;
      const drop = new Set(ids);
      const queue: number[] = [];
      let newIndex = -1;
      state.queue.forEach((id, i) => {
        const isCurrent = i === state.queueIndex;
        if (drop.has(id) && !isCurrent && id !== keepId) return;
        if (isCurrent) newIndex = queue.length;
        queue.push(id);
      });
      if (newIndex < 0) newIndex = queue.length > 0 ? 0 : -1;
      return {
        queue,
        queueIndex: clamp(newIndex, -1, queue.length - 1),
        currentTrackId: queue[newIndex] ?? state.currentTrackId,
      };
    }),

  next: () => {
    const { queue, queueIndex, repeatMode, shuffle } = get();
    if (queue.length === 0) return;
    if (shuffle && queue.length > 1) {
      let index = queueIndex;
      while (index === queueIndex) index = Math.floor(Math.random() * queue.length);
      set({ queueIndex: index, currentTrackId: queue[index] ?? null, positionSec: 0 });
      return;
    }
    let index = queueIndex + 1;
    if (index >= queue.length) {
      if (repeatMode === "queue") index = 0;
      else return;
    }
    set({ queueIndex: index, currentTrackId: queue[index] ?? null, positionSec: 0 });
  },
  prev: () => {
    const { queue, queueIndex, positionSec } = get();
    if (queue.length === 0) return;
    if (positionSec > 3) {
      get().seekTo(0);
      return;
    }
    const index = Math.max(0, queueIndex - 1);
    set({ queueIndex: index, currentTrackId: queue[index] ?? null, positionSec: 0 });
  },
  peekNextId: () => {
    const { queue, queueIndex, repeatMode, shuffle } = get();
    if (queue.length === 0) return null;
    if (shuffle) {
      for (let attempt = 0; attempt < 8; attempt++) {
        const index = Math.floor(Math.random() * queue.length);
        if (index !== queueIndex) return queue[index] ?? null;
      }
      return null;
    }
    const index = queueIndex + 1;
    if (index < queue.length) return queue[index] ?? null;
    return repeatMode === "queue" ? (queue[0] ?? null) : null;
  },
}));
