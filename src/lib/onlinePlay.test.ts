import { describe, expect, it } from "vitest";
import { identityId, pickFromRow, pickKey } from "./onlinePick";

// P9.7/P9.8: cache-key identity for the prime maps (pure module — no
// tauri/zustand/window imports).

describe("pickKey", () => {
  it("prefers the iTunes id when present", () => {
    expect(
      pickKey({ itunesId: 1438132740, mbid: "itunes:1438132740", title: "T" }),
    ).toBe("itunes:1438132740");
  });

  it("uses a real MusicBrainz id as mb:", () => {
    expect(pickKey({ mbid: "647fc53c-0b5f-4c6e-9d0e-1a2b3c4d5e6f", title: "T" })).toBe(
      "mb:647fc53c-0b5f-4c6e-9d0e-1a2b3c4d5e6f",
    );
  });

  it("falls back to metadata when only an itunes: mbid string exists", () => {
    expect(pickKey({ mbid: "itunes:99", title: "Song", artist: "Artist" })).toBe(
      "meta:Song|Artist",
    );
  });

  it("falls back to metadata when there is no id at all", () => {
    expect(pickKey({ title: "Song", artist: "Artist" })).toBe("meta:Song|Artist");
  });
});

describe("identityId", () => {
  it("stores iTunes ids as itunes:<id>", () => {
    expect(identityId({ itunesId: 42, title: "T" })).toBe("itunes:42");
  });

  it("passes MusicBrainz ids through unchanged", () => {
    expect(identityId({ mbid: "abc-123", title: "T" })).toBe("abc-123");
  });

  it("is empty without any id", () => {
    expect(identityId({ title: "T" })).toBe("");
  });
});

describe("pickFromRow", () => {
  it("maps an overlay result row to an OnlinePick", () => {
    expect(
      pickFromRow({
        mbid: "itunes:9",
        itunesId: 9,
        title: "Song",
        sub: "Artist",
        durationMs: 180000,
        artworkUrl: "https://is1-ssl.mzstatic.com/image/thumb/x/100x100bb.jpg",
      }),
    ).toEqual({
      mbid: "itunes:9",
      itunesId: 9,
      title: "Song",
      artist: "Artist",
      durationMs: 180000,
      artworkUrl: "https://is1-ssl.mzstatic.com/image/thumb/x/100x100bb.jpg",
    });
  });

  it("keeps rows without ids/optional fields", () => {
    expect(pickFromRow({ title: "Song" })).toEqual({
      mbid: undefined,
      itunesId: undefined,
      title: "Song",
      artist: undefined,
      durationMs: undefined,
      artworkUrl: undefined,
    });
  });
});
