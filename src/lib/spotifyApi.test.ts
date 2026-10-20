import { describe, expect, it, vi } from "vitest";

// Pure-mapping tests: mock the Tauri boundary (api.ts reads `window` at
// import time — node/vitest has none).
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@/lib/api", () => ({ inTauri: false }));

import { identityId, pickKey, type OnlinePick } from "./onlinePick";
import { spotifyCandidates, type SpotifyPlaylist } from "./spotifyApi";

describe("spotifyCandidates (P17)", () => {
  const pl: SpotifyPlaylist = {
    name: "List",
    total: 3,
    tracks: [
      {
        uri: "spotify:track:AAAA",
        title: "Song",
        artist: "Artist",
        durationMs: 100000,
        artworkUrl: "https://img/art",
      },
      { uri: "spotify:track:BBBB", title: "   ", artist: "Artist", durationMs: null },
      { uri: "spotify:track:CCCC", title: "Two", artist: "B", durationMs: 200000, artworkUrl: null },
    ],
  };

  it("drops untitled rows and maps the rest to mbid candidates", () => {
    const cands = spotifyCandidates(pl);
    expect(cands).toHaveLength(2);
    expect(cands[0]).toEqual({
      mbid: "spotify:track:AAAA",
      title: "Song",
      artist: "Artist",
      durationMs: 100000,
      artworkUrl: "https://img/art",
    });
    expect(cands[1].mbid).toBe("spotify:track:CCCC");
  });

  it("P21: artwork flows through (null stays null for coverless rows)", () => {
    const cands = spotifyCandidates(pl);
    expect(cands[0].artworkUrl).toBe("https://img/art");
    expect(cands[1].artworkUrl).toBeNull();
  });

  it("spotify picks get a stable non-itunes identity (D18 collision rule)", () => {
    const [cand] = spotifyCandidates(pl);
    const pick: OnlinePick = { mbid: cand.mbid ?? undefined, title: cand.title, artist: cand.artist };
    expect(identityId(pick)).toBe("spotify:track:AAAA");
    expect(pickKey(pick)).toBe("mb:spotify:track:AAAA");
  });
});
