import { describe, expect, it } from "vitest";
import type { Track } from "@/lib/types";
import {
  decadeTracks,
  forgottenFavorites,
  genreMix,
  onRepeat,
  playScore,
  recencyWeight,
  seedQueries,
  topArtists,
  topDecades,
  tracksByArtist,
} from "@/lib/taste";

const NOW = Date.parse("2026-10-17T12:00:00Z");
const DAY = 86_400_000;
let seq = 0;

function track(partial: Partial<Track> & { title: string }): Track {
  return {
    id: ++seq,
    source: "local",
    artist: "Unknown Artist",
    addedAt: "2026-01-01T00:00:00Z",
    playCount: 0,
    ...partial,
  };
}

const iso = (daysAgo: number) => new Date(NOW - daysAgo * DAY).toISOString();

describe("recencyWeight / playScore", () => {
  it("halves weight roughly every 14 days and ignores missing stamps", () => {
    expect(recencyWeight(iso(0), NOW)).toBe(1);
    expect(recencyWeight(iso(14), NOW)).toBeCloseTo(0.5, 5);
    expect(recencyWeight(null, NOW)).toBe(0);
    expect(recencyWeight("not-a-date", NOW)).toBe(0);
  });

  it("scales plays by recency — a fresh play beats a stale pile", () => {
    const fresh = track({ title: "Fresh", playCount: 3, lastPlayedAt: iso(0) });
    const stale = track({ title: "Stale", playCount: 30, lastPlayedAt: iso(365) });
    expect(fresh.playCount).toBeLessThan(stale.playCount);
    expect(playScore(fresh, NOW)).toBeGreaterThan(playScore(stale, NOW));
  });
});

describe("topArtists", () => {
  it("prefers recent plays over old volume", () => {
    const tracks = [
      track({ title: "a", artist: "Recent Band", playCount: 4, lastPlayedAt: iso(1) }),
      track({ title: "b", artist: "Old Band", playCount: 40, lastPlayedAt: iso(300) }),
    ];
    expect(topArtists(tracks, NOW).map((a) => a.artist)).toEqual(["Recent Band", "Old Band"]);
  });

  it("falls back to track count when nothing has been played", () => {
    const tracks = [
      track({ title: "a", artist: "Big Artist" }),
      track({ title: "b", artist: "Big Artist" }),
      track({ title: "c", artist: "Small Artist" }),
    ];
    expect(topArtists(tracks, NOW)[0]).toEqual({ artist: "Big Artist", score: 0, tracks: 2 });
  });

  it("groups case-insensitively, keeps first spelling, drops blank artists", () => {
    const tracks = [
      track({ title: "a", artist: "Massive Attack", playCount: 1, lastPlayedAt: iso(0) }),
      track({ title: "b", artist: "massive attack", playCount: 5, lastPlayedAt: iso(0) }),
      track({ title: "c", artist: "   " }),
    ];
    const top = topArtists(tracks, NOW);
    expect(top).toHaveLength(1);
    expect(top[0].artist).toBe("Massive Attack");
    expect(top[0].tracks).toBe(2);
  });
});

describe("genreMix", () => {
  it("merges case-variants and skips unknown-ish tags", () => {
    const tracks = [
      track({ title: "a", genre: "Electronic", playCount: 2, lastPlayedAt: iso(0) }),
      track({ title: "b", genre: "electronic", playCount: 3, lastPlayedAt: iso(0) }),
      track({ title: "c", genre: "Unknown" }),
      track({ title: "d", genre: "" }),
      track({ title: "e", genre: null }),
    ];
    const mix = genreMix(tracks, NOW);
    expect(mix).toHaveLength(1);
    expect(mix[0].genre).toBe("Electronic");
    expect(mix[0].tracks).toBe(2);
    expect(mix[0].score).toBeGreaterThan(0);
  });
});

describe("onRepeat", () => {
  it("keeps only plays inside the window, loved first", () => {
    const tracks = [
      track({ title: "hot", playCount: 2, lastPlayedAt: iso(1) }),
      track({ title: "warm", playCount: 9, lastPlayedAt: iso(10) }),
      track({ title: "cold", playCount: 99, lastPlayedAt: iso(60) }),
      track({ title: "never", playCount: 5 }),
    ];
    expect(onRepeat(tracks, NOW).map((t) => t.title)).toEqual(["warm", "hot"]);
  });

  it("returns empty on a fresh library (shelf hides)", () => {
    expect(onRepeat([track({ title: "x" })], NOW)).toEqual([]);
  });
});

describe("forgottenFavorites", () => {
  it("needs both minimum plays and enough neglect", () => {
    const tracks = [
      track({ title: "gem", playCount: 8, lastPlayedAt: iso(90) }),
      track({ title: "casual", playCount: 1, lastPlayedAt: iso(90) }),
      track({ title: "recent", playCount: 8, lastPlayedAt: iso(3) }),
      track({ title: "unplayed", playCount: 8 }),
    ];
    expect(forgottenFavorites(tracks, NOW).map((t) => t.title)).toEqual(["gem"]);
  });

  it("orders by raw play count", () => {
    const tracks = [
      track({ title: "b", playCount: 5, lastPlayedAt: iso(40) }),
      track({ title: "a", playCount: 12, lastPlayedAt: iso(45) }),
    ];
    expect(forgottenFavorites(tracks, NOW).map((t) => t.title)).toEqual(["a", "b"]);
  });
});

describe("tracksByArtist", () => {
  it("matches case-insensitively and sorts by playScore", () => {
    const tracks = [
      track({ title: "one", artist: "Boards of Canada", playCount: 1, lastPlayedAt: iso(2) }),
      track({ title: "two", artist: "boards of canada", playCount: 6, lastPlayedAt: iso(2) }),
      track({ title: "three", artist: "Aphex Twin", playCount: 9, lastPlayedAt: iso(2) }),
    ];
    expect(tracksByArtist(tracks, "BOARDS OF CANADA", NOW).map((t) => t.title)).toEqual([
      "two",
      "one",
    ]);
  });
});

describe("topDecades", () => {
  it("needs 3 tracks per decade and ranks by love", () => {
    const tracks = [
      ...[1, 2, 3].map((n) =>
        track({ title: `90s-${n}`, year: 1995, playCount: 1, lastPlayedAt: iso(1) }),
      ),
      ...[1, 2].map((n) => track({ title: `00s-${n}`, year: 2003 })),
      ...[1, 2, 3].map((n) => track({ title: `10s-${n}`, year: 2015 })),
    ];
    const decades = topDecades(tracks, NOW);
    expect(decades).toEqual([1990, 2010]);
    expect(topDecades([track({ title: "solo", year: 1991 })], NOW)).toEqual([]);
  });

  it("decadeTracks keeps only that decade, loved first", () => {
    const tracks = [
      track({ title: "loved", year: 1996, playCount: 4, lastPlayedAt: iso(1) }),
      track({ title: "other", year: 1991, playCount: 1, lastPlayedAt: iso(1) }),
      track({ title: "wrong", year: 2001, playCount: 9, lastPlayedAt: iso(1) }),
      track({ title: "undated" }),
    ];
    expect(decadeTracks(tracks, 1990, NOW).map((t) => t.title)).toEqual(["loved", "other"]);
  });
});

describe("seedQueries", () => {
  it("seeds with top artists, then the dominant genre", () => {
    const tracks = [
      track({ title: "a", artist: "Daft Punk", genre: "Electronic", playCount: 5, lastPlayedAt: iso(1) }),
      track({ title: "b", artist: "Daft Punk", genre: "Electronic", playCount: 3, lastPlayedAt: iso(2) }),
      track({ title: "c", artist: "Boards of Canada", genre: "Electronic" }),
    ];
    expect(seedQueries(tracks, NOW)).toEqual(["Daft Punk", "Boards of Canada", "Electronic"]);
  });

  it("returns nothing for an empty library", () => {
    expect(seedQueries([], NOW)).toEqual([]);
  });
});
