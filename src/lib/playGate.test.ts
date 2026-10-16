import { describe, expect, it } from "vitest";
import { shouldRecordPlay } from "./playGate";

// P10: exactly one record per deliberate start — restore/pause/resume must
// not inflate counts; playQueue (incl. same-track replay), next, and
// crossfade track swaps must count.

const idle = { currentTrackId: null, isPlaying: false, playEpoch: 0 };

describe("shouldRecordPlay", () => {
  it("records playQueue from idle", () => {
    expect(shouldRecordPlay(idle, { currentTrackId: 7, isPlaying: true, playEpoch: 1 })).toBe(true);
  });

  it("records replaying the SAME track (epoch bump, id unchanged)", () => {
    expect(
      shouldRecordPlay(
        { currentTrackId: 7, isPlaying: true, playEpoch: 1 },
        { currentTrackId: 7, isPlaying: true, playEpoch: 2 },
      ),
    ).toBe(true);
  });

  it("records switching tracks while already playing (crossfade/next)", () => {
    expect(
      shouldRecordPlay(
        { currentTrackId: 5, isPlaying: true, playEpoch: 1 },
        { currentTrackId: 9, isPlaying: true, playEpoch: 1 },
      ),
    ).toBe(true);
  });

  it("does not record session restore (track set while paused)", () => {
    expect(shouldRecordPlay(idle, { currentTrackId: 7, isPlaying: false, playEpoch: 0 })).toBe(false);
  });

  it("does not record pause or resume of the same track", () => {
    const playing = { currentTrackId: 7, isPlaying: true, playEpoch: 1 };
    expect(shouldRecordPlay(playing, { ...playing, isPlaying: false })).toBe(false);
    const paused = { currentTrackId: 7, isPlaying: false, playEpoch: 1 };
    expect(shouldRecordPlay(paused, { ...paused, isPlaying: true })).toBe(false);
  });

  it("does not record next() while paused", () => {
    expect(
      shouldRecordPlay(
        { currentTrackId: 7, isPlaying: false, playEpoch: 1 },
        { currentTrackId: 8, isPlaying: false, playEpoch: 1 },
      ),
    ).toBe(false);
  });

  it("does not record a null track (queue cleared)", () => {
    expect(
      shouldRecordPlay(
        { currentTrackId: 7, isPlaying: true, playEpoch: 1 },
        { currentTrackId: null, isPlaying: false, playEpoch: 1 },
      ),
    ).toBe(false);
  });
});
