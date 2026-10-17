import { describe, expect, it } from "vitest";
import {
  findLibraryMatch,
  normalizeMeta,
  planPicks,
  recCountFor,
  sampleArtists,
} from "./recommend";

describe("recCountFor (P15)", () => {
  it("returns 0 for an empty playlist", () => {
    expect(recCountFor(0)).toBe(0);
    expect(recCountFor(-3)).toBe(0);
  });

  it("floors at 2 and is ceil(size/2) otherwise", () => {
    expect(recCountFor(1)).toBe(2);
    expect(recCountFor(3)).toBe(2);
    expect(recCountFor(4)).toBe(2);
    expect(recCountFor(5)).toBe(3);
    expect(recCountFor(9)).toBe(5);
  });

  it("caps at 10", () => {
    expect(recCountFor(40)).toBe(10);
    expect(recCountFor(1000)).toBe(10);
  });
});

describe("normalizeMeta (P15)", () => {
  it("lowercases and collapses whitespace", () => {
    expect(normalizeMeta("  Hey   Jude ", "the BEATLES")).toBe(
      "hey jude the beatles",
    );
  });

  it("tolerates a missing artist", () => {
    expect(normalizeMeta("Solo")).toBe("solo");
  });
});

describe("sampleArtists (P15)", () => {
  const tracks = [
    { artist: "A" },
    { artist: "A" },
    { artist: "B" },
    { artist: "  " },
    { artist: null },
    { artist: "C" },
  ];

  it("dedupes and drops empty artists", () => {
    expect(sampleArtists(tracks, 10, () => 0).sort()).toEqual(["A", "B", "C"]);
  });

  it("respects max with a deterministic rng", () => {
    expect(sampleArtists(tracks, 2, () => 0.99)).toHaveLength(2);
    expect(sampleArtists(tracks, 0, () => 0.5)).toEqual([]);
  });
});

describe("planPicks (P15)", () => {
  const cand = (id: number, title: string, artist: string) => ({
    itunesId: id,
    title,
    artist,
  });

  it("round-robins across seed groups and stops at count", () => {
    const g1 = [cand(1, "t1", "x"), cand(2, "t2", "x"), cand(3, "t3", "x")];
    const g2 = [cand(4, "t4", "y"), cand(5, "t5", "y")];
    const picks = planPicks([g1, g2], new Set(), 3);
    expect(picks.map((p) => p.itunesId)).toEqual([1, 4, 2]);
  });

  it("excludes playlist members by normalized title+artist", () => {
    const group = [cand(1, "Hey Jude", "The Beatles"), cand(2, "Other", "Z")];
    const exclude = new Set([normalizeMeta("hey jude", "the beatles")]);
    const picks = planPicks([group], exclude, 5);
    expect(picks.map((p) => p.itunesId)).toEqual([2]);
  });

  it("drops in-pool dupes across groups", () => {
    const a = [cand(1, "Song", "Same Artist")];
    const b = [cand(9, "song", "same   artist"), cand(2, "Fresh", "Other")];
    const picks = planPicks([a, b], new Set(), 5);
    expect(picks.map((p) => p.itunesId)).toEqual([1, 2]);
  });

  it("exits when every candidate is excluded (no infinite loop)", () => {
    const group = [cand(1, "A", "x"), cand(2, "B", "x")];
    const exclude = new Set([normalizeMeta("a", "x"), normalizeMeta("b", "x")]);
    expect(planPicks([group], exclude, 3)).toEqual([]);
  });
});

describe("findLibraryMatch (P15)", () => {
  const tracks = [
    { id: 11, title: "Hey Jude", artist: "The Beatles", mbRecordingId: "abc" },
    { id: 12, title: "Other", artist: "Zed", mbRecordingId: null },
    { id: 13, title: "Cached", artist: "A", mbRecordingId: "itunes:77" },
  ];

  it("matches by itunes identity first", () => {
    expect(
      findLibraryMatch({ itunesId: 77, title: "ignored", artist: "zzz" }, tracks),
    ).toBe(13);
  });

  it("falls back to normalized title+artist", () => {
    expect(
      findLibraryMatch(
        { itunesId: 5, title: "  hey   JUDE ", artist: "the beatles" },
        tracks,
      ),
    ).toBe(11);
  });

  it("returns null when nothing matches", () => {
    expect(
      findLibraryMatch({ itunesId: 5, title: "New", artist: "Nobody" }, tracks),
    ).toBeNull();
  });
});

describe("findLibraryMatch (P17 Spotify identity)", () => {
  const tracks = [
    {
      id: 21,
      title: "Song One",
      artist: "Artist A",
      mbRecordingId: "spotify:track:AAAAAAAAAAAAAAAAAAAAA1",
    },
    { id: 22, title: "Other", artist: "Zed", mbRecordingId: "itunes:9" },
  ];

  it("matches by mbid when there is no itunesId", () => {
    expect(
      findLibraryMatch(
        { mbid: "spotify:track:AAAAAAAAAAAAAAAAAAAAA1", title: "ignored", artist: "zz" },
        tracks,
      ),
    ).toBe(21);
  });

  it("itunes identity never collides with a spotify-identity row", () => {
    expect(findLibraryMatch({ itunesId: 9, title: "ignored", artist: "zz" }, tracks)).toBe(22);
  });

  it("no identity at all still matches by normalized meta", () => {
    expect(findLibraryMatch({ title: "  other ", artist: "zed" }, tracks)).toBe(22);
  });
});
