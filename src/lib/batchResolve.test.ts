import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Candidate } from "@/lib/resolverApi";
import type { OnlinePick } from "@/lib/onlinePick";

/** P20.2 (D34): chunking/order, ≤3 concurrency, latch + session latch,
    cache seeding, per-item chain fallback, prime/inflight skip. Module state
    (latch + prime caches) is reset per test via vi.resetModules + dynamic
    imports; resolverApi is mocked so resolveOnline/resolveBatch are the only
    network surface. */

vi.mock("@/lib/resolverApi", () => ({
  resolveQuery: vi.fn(),
  resolveOnline: vi.fn(),
  resolveBatch: vi.fn(),
}));

type BatchMod = typeof import("@/lib/batchResolve");
type PlayMod = typeof import("@/lib/onlinePlay");
type ApiMod = typeof import("@/lib/resolverApi");

let batch: BatchMod;
let play: PlayMod;
let api: ApiMod;

const tick = () => new Promise((r) => setTimeout(r, 0));

function pick(i: number): OnlinePick {
  return { itunesId: i, title: `Song ${i}`, artist: `Artist ${i}`, durationMs: 180_000 };
}

function cand(marker: string): Candidate {
  return { urlOrPath: `url:${marker}`, mime: null, expiresAt: null, source: "ytdlp", confidence: 0.9 };
}

beforeEach(async () => {
  vi.resetModules();
  batch = await import("@/lib/batchResolve");
  play = await import("@/lib/onlinePlay");
  api = await import("@/lib/resolverApi");
});

describe("resolveManyPicks chunking", () => {
  it("splits into 8-query chunks in pick order and seeds the prime cache", async () => {
    vi.mocked(api.resolveBatch).mockImplementation(async (qs) => ({
      latched: false,
      items: qs.map((q) => ({ ok: true, candidate: cand(q.title), error: null })),
    }));
    const picks = Array.from({ length: 10 }, (_, i) => pick(i));

    batch.resolveManyPicks(picks);
    await tick();

    expect(api.resolveBatch).toHaveBeenCalledTimes(2);
    const first = vi.mocked(api.resolveBatch).mock.calls[0][0];
    const second = vi.mocked(api.resolveBatch).mock.calls[1][0];
    expect(first).toHaveLength(8);
    expect(second).toHaveLength(2);
    expect(first.map((q) => q.title)).toEqual(picks.slice(0, 8).map((p) => p.title));

    const resolved = await play.resolveOnlinePick(picks[0]);
    expect(resolved?.urlOrPath).toBe("url:Song 0");
    expect(api.resolveOnline).not.toHaveBeenCalled();
  });

  it("never runs more than 3 chunks concurrently", async () => {
    let active = 0;
    let maxActive = 0;
    const releases: Array<() => void> = [];
    vi.mocked(api.resolveBatch).mockImplementation((qs) => {
      active++;
      maxActive = Math.max(maxActive, active);
      return new Promise((resolve) => {
        releases.push(() => {
          active--;
          resolve({
            latched: false,
            items: qs.map((q) => ({ ok: true, candidate: cand(q.title), error: null })),
          });
        });
      });
    });
    const picks = Array.from({ length: 28 }, (_, i) => pick(i)); // 4 chunks

    batch.resolveManyPicks(picks);
    await tick();
    expect(api.resolveBatch).toHaveBeenCalledTimes(3); // window full, 4th queued

    while (releases.length > 0) {
      releases.shift()!();
      await tick();
    }
    expect(api.resolveBatch).toHaveBeenCalledTimes(4);
    expect(maxActive).toBeLessThanOrEqual(3);
    const last = await play.resolveOnlinePick(picks[27]);
    expect(last?.urlOrPath).toBe("url:Song 27");
  });

  it("skips primed and in-flight keys", async () => {
    vi.mocked(api.resolveBatch).mockImplementation(async (qs) => ({
      latched: false,
      items: qs.map((q) => ({ ok: true, candidate: cand(q.title), error: null })),
    }));
    const [a, b, c, d] = [pick(100), pick(101), pick(102), pick(103)];

    batch.resolveManyPicks([a, b]);
    await tick(); // both now cached
    vi.mocked(api.resolveBatch).mockClear();

    // d parked on a hanging chain prime → in-flight.
    vi.mocked(api.resolveOnline).mockReturnValue(new Promise<Candidate | null>(() => {}));
    play.primeOnlinePick(d);
    await tick();

    batch.resolveManyPicks([a, b, c, d]);
    await tick();
    expect(api.resolveBatch).toHaveBeenCalledTimes(1);
    expect(vi.mocked(api.resolveBatch).mock.calls[0][0].map((q) => q.title)).toEqual([
      "Song 102",
    ]);
  });
});

describe("latch behavior", () => {
  it("stops dispatching queued chunks, falls back per pick, and sticks for the session", async () => {
    vi.mocked(api.resolveBatch).mockResolvedValue({
      latched: true,
      items: Array.from({ length: 8 }, () => ({ ok: false, candidate: null, error: "spawn" })),
    });
    vi.mocked(api.resolveOnline).mockImplementation(async (q) => cand(`chain:${q.title}`));
    const picks = Array.from({ length: 30 }, (_, i) => pick(i));

    batch.resolveManyPicks(picks);
    await tick();
    expect(batch.isBatchLatched()).toBe(true);
    expect(api.resolveBatch).toHaveBeenCalledTimes(3); // window dispatched, rest queued

    // A pick from queued chunk 5+ resolves through the single chain.
    const chained = await play.resolveOnlinePick(picks[20]);
    expect(chained?.urlOrPath).toBe("url:chain:Song 20");
    expect(api.resolveOnline).toHaveBeenCalledWith(
      expect.objectContaining({ title: "Song 20" }),
    );

    // Session latch: a fresh save attempts no new chunks.
    batch.resolveManyPicks(Array.from({ length: 8 }, (_, i) => pick(50 + i)));
    await tick();
    expect(api.resolveBatch).toHaveBeenCalledTimes(3);
  });

  it("a batch-miss item still falls back to the single chain", async () => {
    vi.mocked(api.resolveBatch).mockResolvedValue({
      latched: false,
      items: [{ ok: false, candidate: null, error: "no usable video" }],
    });
    vi.mocked(api.resolveOnline).mockResolvedValue(cand("chain-rescued"));

    batch.resolveManyPicks([pick(30)]);
    await tick();
    const got = await play.resolveOnlinePick(pick(30));
    expect(got?.urlOrPath).toBe("url:chain-rescued");
    expect(api.resolveOnline).toHaveBeenCalledTimes(1);
  });

  it("a rejected invoke trips the latch and everything chains from there", async () => {
    vi.mocked(api.resolveBatch).mockRejectedValue(new Error("invoke boom"));
    vi.mocked(api.resolveOnline).mockResolvedValue(cand("chain"));

    batch.resolveManyPicks([pick(40)]);
    await tick();
    expect(batch.isBatchLatched()).toBe(true);
    expect((await play.resolveOnlinePick(pick(40)))?.urlOrPath).toBe("url:chain");

    batch.resolveManyPicks([pick(41)]);
    await tick();
    expect(api.resolveBatch).toHaveBeenCalledTimes(1); // latch held
    expect((await play.resolveOnlinePick(pick(41)))?.urlOrPath).toBe("url:chain");
  });
});

describe("single-flight integration", () => {
  it("two waiters of a batched key share one flight", async () => {
    vi.mocked(api.resolveBatch).mockImplementation(async (qs) => ({
      latched: false,
      items: qs.map((q) => ({ ok: true, candidate: cand(q.title), error: null })),
    }));
    const p = pick(60);
    batch.resolveManyPicks([p]);
    await tick();

    const [x, y] = await Promise.all([play.resolveOnlinePick(p), play.resolveOnlinePick(p)]);
    expect(x?.urlOrPath).toBe("url:Song 60");
    expect(y).toBe(x);
    expect(api.resolveOnline).not.toHaveBeenCalled();
  });
});
