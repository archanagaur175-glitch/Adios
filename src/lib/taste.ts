/** P16: pure taste engine — scores the library by play history to feed the
    Home taste shelves (C) and seed queries for the online sections (A/D).
    Import-free (types only) so it's unit-testable in node. */

import type { Track } from "@/lib/types";

const DAY_MS = 86_400_000;
/** On Repeat: plays inside this window qualify. */
export const REPEAT_WINDOW_DAYS = 14;
/** Forgotten favorites: at least this many plays… */
export const FORGOTTEN_MIN_PLAYS = 2;
/** …and untouched for at least this long. */
export const FORGOTTEN_AFTER_DAYS = 30;
/** A decade shelf needs at least this many tracks to be worth showing. */
const DECADE_MIN_TRACKS = 3;

export interface ArtistScore {
  /** First-seen display spelling (grouping is case-insensitive). */
  artist: string;
  /** Σ playScore across the artist's tracks. */
  score: number;
  /** Library tracks by this artist (fallback ordering when nobody played). */
  tracks: number;
}

export interface GenreScore {
  genre: string;
  score: number;
  tracks: number;
}

/** Recency decay: 1 for just-played, halving roughly every 14 days. Never
    played / unparsable → 0. */
export function recencyWeight(lastPlayedAt?: string | null, now = Date.now()): number {
  if (!lastPlayedAt) return 0;
  const t = Date.parse(lastPlayedAt);
  if (Number.isNaN(t)) return 0;
  const ageDays = Math.max(0, (now - t) / DAY_MS);
  return 1 / (1 + ageDays / 14);
}

/** Plays scaled by recency — one play today beats ten from last year
    (w=1 fresh, ≈0.04 after 300 days, so the old pile never wins). */
export function playScore(t: Track, now = Date.now()): number {
  return (t.playCount ?? 0) * recencyWeight(t.lastPlayedAt, now);
}

function isoDaysAgo(days: number, now: number): number {
  return now - days * DAY_MS;
}

/** Distinct artists ranked by playScore; ties (including the all-zero,
    nobody-has-played-yet library) fall back to track count, so a fresh
    install still gets its most-represented artists as seeds. */
export function topArtists(tracks: Track[], now = Date.now(), max = 5): ArtistScore[] {
  const byKey = new Map<string, ArtistScore>();
  for (const t of tracks) {
    const name = t.artist?.trim();
    if (!name) continue;
    const key = name.toLowerCase();
    const entry = byKey.get(key) ?? { artist: name, score: 0, tracks: 0 };
    if (entry.tracks === 0) entry.artist = name;
    entry.tracks++;
    entry.score += playScore(t, now);
    byKey.set(key, entry);
  }
  return [...byKey.values()]
    .sort((a, b) => b.score - a.score || b.tracks - a.tracks || a.artist.localeCompare(b.artist))
    .slice(0, max);
}

/** Genres ranked the same way as artists (freeform tags + iTunes buckets
    group case-insensitively; empty/"unknown" values are dropped). */
export function genreMix(tracks: Track[], now = Date.now(), max = 5): GenreScore[] {
  const byKey = new Map<string, GenreScore>();
  for (const t of tracks) {
    const name = t.genre?.trim();
    if (!name || /^(unknown|none|other)$/i.test(name)) continue;
    const key = name.toLowerCase();
    const entry = byKey.get(key) ?? { genre: name, score: 0, tracks: 0 };
    if (entry.tracks === 0) entry.genre = name;
    entry.tracks++;
    entry.score += playScore(t, now);
    byKey.set(key, entry);
  }
  return [...byKey.values()]
    .sort((a, b) => b.score - a.score || b.tracks - a.tracks || a.genre.localeCompare(b.genre))
    .slice(0, max);
}

/** Played inside REPEAT_WINDOW_DAYS, most-loved first. Empty = shelf hidden. */
export function onRepeat(tracks: Track[], now = Date.now(), limit = 8): Track[] {
  const cutoff = isoDaysAgo(REPEAT_WINDOW_DAYS, now);
  return tracks
    .filter((t) => {
      if (!t.playCount || !t.lastPlayedAt) return false;
      const t0 = Date.parse(t.lastPlayedAt);
      return !Number.isNaN(t0) && t0 >= cutoff;
    })
    .sort((a, b) => playScore(b, now) - playScore(a, now))
    .slice(0, limit);
}

/** Loved once, long abandoned: heavy plays but no recent play. */
export function forgottenFavorites(tracks: Track[], now = Date.now(), limit = 8): Track[] {
  const cutoff = isoDaysAgo(FORGOTTEN_AFTER_DAYS, now);
  return tracks
    .filter((t) => {
      if ((t.playCount ?? 0) < FORGOTTEN_MIN_PLAYS || !t.lastPlayedAt) return false;
      const t0 = Date.parse(t.lastPlayedAt);
      return !Number.isNaN(t0) && t0 < cutoff;
    })
    .sort((a, b) => b.playCount - a.playCount || a.title.localeCompare(b.title))
    .slice(0, limit);
}

/** All tracks by one artist, loved first (case-insensitive match). */
export function tracksByArtist(tracks: Track[], artist: string, now = Date.now()): Track[] {
  const key = artist.trim().toLowerCase();
  return tracks
    .filter((t) => t.artist?.trim().toLowerCase() === key)
    .sort((a, b) => playScore(b, now) - playScore(a, now));
}

/** Library decades (from `year`) with enough tracks, best-loved first.
    Returns the decade start: 1990 for anything 1990–1999. */
export function topDecades(tracks: Track[], now = Date.now(), max = 2): number[] {
  const byDecade = new Map<number, { tracks: number; score: number }>();
  for (const t of tracks) {
    if (!t.year) continue;
    const decade = Math.floor(t.year / 10) * 10;
    const entry = byDecade.get(decade) ?? { tracks: 0, score: 0 };
    entry.tracks++;
    entry.score += playScore(t, now);
    byDecade.set(decade, entry);
  }
  return [...byDecade.entries()]
    .filter(([, e]) => e.tracks >= DECADE_MIN_TRACKS)
    .sort((a, b) => b[1].score - a[1].score || b[0] - a[0])
    .slice(0, max)
    .map(([decade]) => decade);
}

/** Tracks from `decade` (start year, e.g. 1990), loved first, capped. */
export function decadeTracks(tracks: Track[], decade: number, now = Date.now(), limit = 8): Track[] {
  return tracks
    .filter((t) => t.year && Math.floor(t.year / 10) * 10 === decade)
    .sort((a, b) => playScore(b, now) - playScore(a, now))
    .slice(0, limit);
}

/** iTunes seed queries for Picked for you / Discover new: top artists first,
    then the dominant genre; falls back to nothing on an empty library. */
export function seedQueries(tracks: Track[], now = Date.now(), max = 3): string[] {
  const out: string[] = [];
  for (const a of topArtists(tracks, now, max)) out.push(a.artist);
  if (out.length < max) {
    for (const g of genreMix(tracks, now, max)) {
      if (!out.some((q) => q.toLowerCase() === g.genre.toLowerCase())) out.push(g.genre);
      if (out.length >= max) break;
    }
  }
  return out.slice(0, max);
}
