import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Candidate } from "@/lib/resolverApi";
import type { RecCandidate } from "@/lib/recommend";
import type { Track } from "@/lib/types";

/** P20.2 (D34): save-engine wiring — prefetch fires for unresolved picks
    only (library fast-path skips it), progress stays monotonic and
    per-item, failures are skipped, single-flight holds. onlinePlay is
    partially mocked (real caches + runChain, mocked resolve/persist) so the
    pump runs while the loop observes mock results. */

vi.mock("@/lib/playlistApi", () => ({
  createPlaylist: vi.fn(),
  addPlaylistTrack: vi.fn(),
}));
vi.mock("@/lib/resolverApi", () => ({
  resolveQuery: vi.fn(),
  resolveBatch: vi.fn(),
}));
vi.mock("@/lib/onlinePlay", async (importOriginal) => {
  const orig = await importOriginal<typeof import("@/lib/onlinePlay")>();
  return { ...orig, resolveOnlinePick: vi.fn(), persistPick: vi.fn() };
});

type SaveMod = typeof import("@/lib/playlistSave");
type BatchMod = typeof import("@/lib/batchResolve");
type PlayMod = typeof import("@/lib/onlinePlay");
type ApiMod = typeof import("@/lib/resolverApi");
type PlistApi = typeof import("@/lib/playlistApi");
type LibMod = typeof import("@/store/libraryStore");

let save: SaveMod;
let batch: BatchMod;
let play: PlayMod;
let api: ApiMod;
let plist: PlistApi;
let lib: LibMod;

const tick = () => new Promise((r) => setTimeout(r, 0));

function rpick(i: number): RecCandidate {
  return { title: `Track ${i}`, artist: `Artist ${i}`, durationMs: 180_000 };
}

function cand(marker: string): Candidate {
  return { urlOrPath: `url:${marker}`, mime: null, expiresAt: null, source: "ytdlp", confidence: 0.9 };
}

beforeEach(async () => {
  vi.resetModules();
  save = await import("@/lib/playlistSave");
  batch = await import("@/lib/batchResolve");
  play = await import("@/lib/onlinePlay");
  api = await import("@/lib/resolverApi");
  plist = await import("@/lib/playlistApi");
  lib = await import("@/store/libraryStore");
  vi.clearAllMocks();
  lib.useLibraryStore.getState().setTracks([]);
  vi.mocked(plist.createPlaylist).mockResolvedValue(42);
  vi.mocked(plist.addPlaylistTrack).mockResolvedValue(true);
  vi.mocked(api.resolveBatch).mockResolvedValue({ latched: true, items: [] });
});

describe("savePicksToPlaylist (P20 pool wiring)", () => {
  it("prefetches unresolved picks as one chunk, persists sequentially, reports 1..N progress", async () => {
    vi.mocked(api.resolveBatch).mockResolvedValue({ latched: false, items: [] });
    vi.mocked(play.resolveOnlinePick).mockImplementation(async (p) => cand(p.title));
    let seq = 0;
    vi.mocked(play.persistPick).mockImplementation(
      async (p) => ({ id: ++seq, source: "stream", title: p.title, artist: p.artist ?? "" }) as unknown as Track,
    );
    const picks = [rpick(1), rpick(2), rpick(3)];
    const progress: Array<[number, number]> = [];

    const res = await save.savePicksToPlaylist("Mix", picks, (done, total) =>
      progress.push([done, total]),
    );

    expect(api.resolveBatch).toHaveBeenCalledTimes(1);
    expect(vi.mocked(api.resolveBatch).mock.calls[0][0].map((q) => q.title)).toEqual([
      "Track 1",
      "Track 2",
      "Track 3",
    ]);
    expect(res).toEqual({ playlistId: 42, name: "Mix", added: 3, skipped: 0 });
    expect(progress).toEqual([
      [1, 3],
      [2, 3],
      [3, 3],
    ]);
    expect(vi.mocked(plist.addPlaylistTrack).mock.calls).toEqual([
      [42, 1],
      [42, 2],
      [42, 3],
    ]);
  });

  it("library fast-path rows skip both prefetch and resolve", async () => {
    lib.useLibraryStore.getState().setTracks([
      { id: 7, source: "stream", title: "Same Song", artist: "Same Artist" } as unknown as Track,
    ]);

    const res = await save.savePicksToPlaylist(
      "Mix",
      [{ title: "Same Song", artist: "Same Artist", durationMs: 180_000 }],
      () => {},
    );

    expect(api.resolveBatch).not.toHaveBeenCalled();
    expect(play.resolveOnlinePick).not.toHaveBeenCalled();
    expect(vi.mocked(plist.addPlaylistTrack).mock.calls).toEqual([[42, 7]]);
    expect(res?.added).toBe(1);
  });

  it("skips failed picks (reject and null alike) but keeps going", async () => {
    vi.mocked(api.resolveBatch).mockResolvedValue({ latched: false, items: [] });
    vi.mocked(play.resolveOnlinePick).mockImplementation(async (p) => {
      if (p.title === "Track 1") throw new Error("chain exploded");
      if (p.title === "Track 2") return null;
      return cand(p.title);
    });
    vi.mocked(play.persistPick).mockResolvedValue({
      id: 9,
      source: "stream",
      title: "Track 3",
      artist: "Artist 3",
    } as unknown as Track);
    const progress: number[] = [];

    const res = await save.savePicksToPlaylist(
      "Mix",
      [rpick(1), rpick(2), rpick(3)],
      (done) => progress.push(done),
    );

    expect(res).toEqual({ playlistId: 42, name: "Mix", added: 1, skipped: 2 });
    expect(progress).toEqual([1, 2, 3]);
    expect(play.persistPick).toHaveBeenCalledTimes(1);
  });

  it("single-flight: a second save while one runs returns null", async () => {
    let release!: (id: number) => void;
    vi.mocked(plist.createPlaylist).mockReturnValue(
      new Promise((r) => {
        release = r;
      }),
    );
    const picks = [rpick(10)];

    const first = save.savePicksToPlaylist("A", picks, () => {});
    const second = await save.savePicksToPlaylist("B", picks, () => {});
    expect(second).toBeNull();
    expect(save.isSaving()).toBe(true);

    release(42);
    const res = await first;
    expect(res?.playlistId).toBe(42);
    expect(save.isSaving()).toBe(false);
    void batch; // module imported for state isolation only
    await tick();
  });
});
