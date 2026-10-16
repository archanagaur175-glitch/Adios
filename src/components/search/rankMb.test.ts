import { describe, expect, it } from "vitest";
import { mbTrackScore, rankMbTracks } from "./rankMb";

const q = "Never Gonna Give You Up Rick Astley";
const real = { title: "Never Gonna Give You Up", artist: "Rick Astley", score: 100 };
const ripoff = { title: "Never Gonna Give You Up (Rick Astley)", artist: "Ghostface Kilobyte", score: 100 };
const karaoke = { title: "Never Gonna Give You Up", artist: "Karaoke Universe", score: 99 };

describe("rankMbTracks (P9.6 ripoff ranking)", () => {
  it("puts the artist-named-in-query recording first even when MB scores tie", () => {
    const ranked = rankMbTracks(q, [ripoff, karaoke, real]);
    expect(ranked[0]).toBe(real);
    expect(ranked[ranked.length - 1]).toBe(karaoke);
  });

  it("demotes cover-farm artists below unknown-but-clean ones", () => {
    const clean = { title: "Never Gonna Give You Up (Rick Astley)", artist: "Basement Sessions", score: 90 };
    const ranked = rankMbTracks(q, [karaoke, clean]);
    expect(ranked[0]).toBe(clean);
  });

  it("keeps MB score as the tiebreaker for equal matches", () => {
    const a = { title: "Song", artist: "Artist", score: 50 };
    const b = { title: "Song", artist: "Artist", score: 90 };
    expect(rankMbTracks("song artist", [a, b])[0]).toBe(b);
  });

  it("is stable for identical inputs", () => {
    const a = { title: "X", artist: "Y", score: 10 };
    const b = { title: "X", artist: "Y", score: 10 };
    expect(rankMbTracks("x y", [a, b])).toEqual([a, b]);
  });

  it("mbTrackScore: artist mention beats title mention", () => {
    expect(mbTrackScore(q, real)).toBeGreaterThan(mbTrackScore(q, ripoff));
  });
});
