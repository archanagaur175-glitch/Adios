import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Candidate } from "@/lib/resolverApi";

/** P19.3/P19.4 (D33): resolve routing — Android picks go to the Kotlin
    yt-dlp plugin (`resolve_android` / `resolve_android_batch`), everything
    else keeps the Rust chain (`resolve_query` / `resolve_batch`). invoke +
    isAndroid are mocked; the adapter must pass queries through unchanged,
    keep the Candidate shape, and forward the batch latch as-is. */

const invokeMock = vi.hoisted(() => vi.fn());
const isAndroidMock = vi.hoisted(() => vi.fn(() => false));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@/lib/api", async (importOriginal) => {
  const orig = await importOriginal<typeof import("@/lib/api")>();
  return { ...orig, isAndroid: isAndroidMock, inTauri: true };
});

type ApiMod = typeof import("@/lib/resolverApi");
let api: ApiMod;

const cand: Candidate = {
  urlOrPath: "https://cdn/x.m4a",
  mime: "audio/mp4",
  expiresAt: null,
  source: "ytdlp",
  confidence: 0.9,
};

beforeEach(async () => {
  vi.resetModules();
  isAndroidMock.mockReturnValue(false);
  invokeMock.mockReset();
  invokeMock.mockResolvedValue(cand);
  api = await import("@/lib/resolverApi");
});

describe("resolveOnline routing (P19.3)", () => {
  it("desktop routes to resolve_query with the query flattened", async () => {
    const out = await api.resolveOnline({
      artist: "A",
      title: "T",
      album: "Al",
      durationMs: 180_000,
    });

    expect(invokeMock).toHaveBeenCalledWith("resolve_query", {
      artist: "A",
      title: "T",
      album: "Al",
      durationMs: 180_000,
    });
    expect(out).toEqual(cand);
  });

  it("Android routes to resolve_android with the same payload", async () => {
    isAndroidMock.mockReturnValue(true);

    const out = await api.resolveOnline({ artist: "A", title: "T", durationMs: null });

    expect(invokeMock).toHaveBeenCalledWith("resolve_android", {
      artist: "A",
      title: "T",
      album: null,
      durationMs: null,
    });
    expect(out).toEqual(cand);
    expect(invokeMock).not.toHaveBeenCalledWith("resolve_query", expect.anything());
  });

  it("defaults album/durationMs to null on both platforms", async () => {
    await api.resolveOnline({ artist: "", title: "T" });
    expect(invokeMock.mock.calls[0][1]).toMatchObject({ album: null, durationMs: null });

    isAndroidMock.mockReturnValue(true);
    await api.resolveOnline({ artist: "", title: "T" });
    expect(invokeMock.mock.calls[1][1]).toMatchObject({ album: null, durationMs: null });
  });

  it("resolveAndroid rejects when invoke errors (callers toast on catch)", async () => {
    invokeMock.mockRejectedValue(new Error("plugin dead"));
    await expect(api.resolveAndroid({ artist: "A", title: "T" })).rejects.toThrow("plugin dead");
  });
});

describe("resolveBatch routing (P19.4)", () => {
  const ok = { latched: false, items: [{ ok: true, candidate: cand, error: null }] };

  it("desktop invokes resolve_batch with the {queries} wrapper", async () => {
    invokeMock.mockResolvedValue(ok);

    const out = await api.resolveBatch([{ artist: "A", title: "T", durationMs: 180_000 }]);

    expect(invokeMock).toHaveBeenCalledWith("resolve_batch", {
      queries: [{ artist: "A", title: "T", album: null, durationMs: 180_000 }],
    });
    expect(out).toEqual(ok);
  });

  it("Android invokes resolve_android_batch with the identical payload", async () => {
    isAndroidMock.mockReturnValue(true);
    invokeMock.mockResolvedValue(ok);

    await api.resolveBatch([{ artist: "A", title: "T" }]);

    expect(invokeMock).toHaveBeenCalledWith("resolve_android_batch", {
      queries: [{ artist: "A", title: "T", album: null, durationMs: null }],
    });
    expect(invokeMock).not.toHaveBeenCalledWith("resolve_batch", expect.anything());
  });

  it("passes the plugin latch through untouched (sequential fallback engages)", async () => {
    isAndroidMock.mockReturnValue(true);
    invokeMock.mockResolvedValue({
      latched: true,
      items: [{ ok: false, candidate: null, error: "plugin init failed" }],
    });

    const out = await api.resolveBatch([{ artist: "A", title: "T" }]);

    expect(out.latched).toBe(true);
    expect(out.items[0]?.ok).toBe(false);
  });
});
