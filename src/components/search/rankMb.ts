/** P9.6 fix #2: MusicBrainz has no popularity field — its Lucene relevance
    happily ranks karaoke/tribute/lullaby "ripoff" recordings above the real
    song (they share title+artist tokens). Client-side re-rank of MB track
    results using the signals the user actually typed:

    - artist named in the query → strong bonus (the real artist's recording)
    - clean title in the query → medium bonus
    - cover-farm artist markers (karaoke, tribute, lullaby…) → penalty
    - MB's own relevance score breaks ties.
    Stable sort; pure function, unit-tested. */

export interface MbTrackLike {
  title: string;
  artist: string;
  score?: number | null;
}

const COVER_MARKERS =
  /karaoke|tribute|lullaby|rock star|acoustic|instrumental|cover|pillow|performed by|made famous|sing[- ]?along|kids|children|piano|orchestra|symphony|remixed by|tribute to/i;

const norm = (s: string): string => s.toLowerCase().replace(/\s+/g, " ").trim();

/** Title without parentheticals: "Song (Live)" → "song". */
function baseTitle(s: string): string {
  return norm(s).replace(/\s*\([^)]*\)/g, " ").replace(/\s+/g, " ").trim();
}

/** 0–100-ish relevance score for one MB track row vs. the user's query. */
export function mbTrackScore(query: string, item: MbTrackLike): number {
  const q = norm(query);
  let s = item.score ?? 0;
  if (q.includes(norm(item.artist))) s += 1000;
  const base = baseTitle(item.title);
  if (base && q.includes(base)) s += 300;
  if (q.includes(norm(item.title))) s += 100;
  if (COVER_MARKERS.test(item.artist)) s -= 400;
  return s;
}

/** Re-rank MB track results best-first (stable). */
export function rankMbTracks<T extends MbTrackLike>(query: string, items: T[]): T[] {
  return items
    .map((item, i) => ({ item, i, s: mbTrackScore(query, item) }))
    .sort((a, b) => b.s - a.s || a.i - b.i)
    .map((x) => x.item);
}
