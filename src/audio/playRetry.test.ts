import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { playWithRetry } from "./playRetry";

type FakeEl = {
  play: ReturnType<typeof vi.fn>;
  readyState: number;
  listeners: Map<string, (() => void)[]>;
  addEventListener(type: string, fn: () => void): void;
  removeEventListener(type: string, fn: () => void): void;
};

function makeEl(readyState: number, outcomes: ("resolve" | "reject")[]): FakeEl {
  const listeners = new Map<string, (() => void)[]>();
  let call = 0;
  return {
    readyState,
    listeners,
    addEventListener(type, fn) {
      listeners.set(type, [...(listeners.get(type) ?? []), fn]);
    },
    removeEventListener(type, fn) {
      listeners.set(type, (listeners.get(type) ?? []).filter((f) => f !== fn));
    },
    play: vi.fn(() => {
      const outcome = outcomes[Math.min(call++, outcomes.length - 1)];
      return outcome === "resolve" ? Promise.resolve() : Promise.reject(new Error("NotAllowed"));
    }),
  };
}

function fireCanPlay(el: FakeEl): void {
  [...(el.listeners.get("canplay") ?? [])].forEach((fn) => fn());
}

const asMedia = (el: FakeEl) => el as unknown as HTMLMediaElement;

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe("playWithRetry", () => {
  it("returns immediately when the first play() resolves", async () => {
    const el = makeEl(0, ["resolve"]);
    await playWithRetry(asMedia(el));
    expect(el.play).toHaveBeenCalledTimes(1);
  });

  it("retries once at canplay when the first play() rejects with no data yet", async () => {
    const el = makeEl(0, ["reject", "resolve"]);
    const done = playWithRetry(asMedia(el));
    await vi.advanceTimersByTimeAsync(10);
    fireCanPlay(el);
    await done;
    expect(el.play).toHaveBeenCalledTimes(2);
  });

  it("does not retry when data was already available (real refusal)", async () => {
    const el = makeEl(3, ["reject"]);
    await expect(playWithRetry(asMedia(el))).rejects.toThrow("NotAllowed");
    expect(el.play).toHaveBeenCalledTimes(1);
  });

  it("keeps its promise bounded: on canplay timeout it attempts once more", async () => {
    const el = makeEl(0, ["reject", "reject"]);
    const done = playWithRetry(asMedia(el));
    const asserted = expect(done).rejects.toThrow("NotAllowed");
    await vi.advanceTimersByTimeAsync(2600);
    await asserted;
    expect(el.play).toHaveBeenCalledTimes(2);
    expect(el.listeners.get("canplay") ?? []).toHaveLength(0);
  });

  it("gives up with the original error if a retry also rejects", async () => {
    const el = makeEl(0, ["reject", "reject"]);
    const done = playWithRetry(asMedia(el));
    const asserted = expect(done).rejects.toThrow("NotAllowed");
    await vi.advanceTimersByTimeAsync(10);
    fireCanPlay(el);
    await asserted;
    expect(el.play).toHaveBeenCalledTimes(2);
  });
});
