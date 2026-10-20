import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Candidate } from "@/lib/resolverApi";

/** P19.3 (D33): resolve routing — Android picks go to the Kotlin yt-dlp
    plugin (`resolve_android`), everything else keeps the Rust chain
    (`resolve_query`). invoke + isAndroid are mocked; adapter must pass the
    query through unchanged and keep the Candidate shape. */

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
