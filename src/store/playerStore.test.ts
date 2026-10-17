import { beforeEach, describe, expect, it } from "vitest";
import { usePlayerStore } from "./playerStore";

const s = () => usePlayerStore.getState();

describe("playerStore queue actions (P15)", () => {
  beforeEach(() => {
    usePlayerStore.setState({
      queue: [],
      queueIndex: -1,
      currentTrackId: null,
      isPlaying: false,
      positionSec: 0,
      durationSec: 0,
    });
  });

  it("insertQueueAt appends at the end and leaves the index alone", () => {
    usePlayerStore.getState().setQueue([1, 2], 0);
    usePlayerStore.getState().insertQueueAt(3, 2);
    expect(s().queue).toEqual([1, 2, 3]);
    expect(s().queueIndex).toBe(0);
  });

  it("insertQueueAt before the current track shifts queueIndex", () => {
    usePlayerStore.getState().setQueue([1, 2, 3], 2);
    usePlayerStore.getState().insertQueueAt(9, 0);
    expect(s().queue).toEqual([9, 1, 2, 3]);
    expect(s().queueIndex).toBe(3);
    expect(s().currentTrackId).toBe(3);
  });

  it("insertQueueAt at the current position also shifts the index", () => {
    usePlayerStore.getState().setQueue([1, 2, 3], 1);
    usePlayerStore.getState().insertQueueAt(9, 1);
    expect(s().queue).toEqual([1, 9, 2, 3]);
    expect(s().queueIndex).toBe(2);
    expect(s().currentTrackId).toBe(2);
  });

  it("insertQueueAt clamps out-of-range indices", () => {
    usePlayerStore.getState().setQueue([1], 0);
    usePlayerStore.getState().insertQueueAt(2, -5);
    usePlayerStore.getState().insertQueueAt(3, 99);
    expect(s().queue).toEqual([2, 1, 3]);
    expect(s().queueIndex).toBe(1);
  });

  it("removeQueueTracks drops matching ids but never the current track", () => {
    usePlayerStore.getState().setQueue([1, 2, 3, 4], 1);
    usePlayerStore.getState().removeQueueTracks([2, 4], 2);
    expect(s().queue).toEqual([1, 2, 3]);
    expect(s().queueIndex).toBe(1);
    expect(s().currentTrackId).toBe(2);
  });

  it("removeQueueTracks before the current track recomputes queueIndex", () => {
    usePlayerStore.getState().setQueue([1, 2, 3, 4], 3);
    usePlayerStore.getState().removeQueueTracks([1, 2], 4);
    expect(s().queue).toEqual([3, 4]);
    expect(s().queueIndex).toBe(1);
    expect(s().currentTrackId).toBe(4);
  });

  it("removeQueueTracks respects keepId even for a non-current track", () => {
    usePlayerStore.getState().setQueue([1, 2, 3], 0);
    usePlayerStore.getState().removeQueueTracks([1, 2], 2);
    expect(s().queue).toEqual([1, 2, 3]);
    expect(s().queueIndex).toBe(0);
    expect(s().currentTrackId).toBe(1);
  });

  it("removeQueueTracks is a no-op on an empty queue", () => {
    usePlayerStore.getState().removeQueueTracks([1, 2], null);
    expect(s().queue).toEqual([]);
    expect(s().queueIndex).toBe(-1);
  });
});
