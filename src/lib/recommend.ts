/** P15: pure helpers for queue-only recommendations (recQueue.ts imports
    these; no tauri/zustand so they're unit-testable in node). */

export interface RecCandidate {
  itunesId: number;
  title: string;
  artist: string;
  durationMs?: number | null;
  artworkUrl?: string | null;
  /** P16: iTunes primaryGenreName — persisted when the pick needs a row. */
  genre?: string | null;
}

/** How many recs to inject for a playlist of `trackCount` tracks:
    clamp(ceil(size/2), 2, 10); empty playlist → 0. */
export function recCountFor(trackCount: number): number {
  if (trackCount <= 0) return 0;
  return Math.min(10, Math.max(2, Math.ceil(trackCount / 2)));
}

/** Lowercase + collapse whitespace — the dedupe/exclusion key. */
export function normalizeMeta(title: string, artist?: string): string {
  return `${title} ${artist ?? ""}`.toLowerCase().replace(/\s+/g, " ").trim();
}

/** Up to `max` distinct non-empty artist names, shuffled (Fisher-Yates). */
export function sampleArtists(
  tracks: { artist?: string | null }[],
  max: number,
  rng: () => number = Math.random,
): string[] {
  const seen = new Set<string>();
  const artists: string[] = [];
  for (const t of tracks) {
    const a = t.artist?.trim();
    if (!a || seen.has(a)) continue;
    seen.add(a);
    artists.push(a);
  }
  for (let i = artists.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    [artists[i], artists[j]] = [artists[j], artists[i]];
  }
  return artists.slice(0, max);
}

/** Round-robin across per-seed result groups, skipping normalized keys in
    `exclude` (playlist members) and already-picked dupes; stops at `count`. */
export function planPicks(
  groups: RecCandidate[][],
  exclude: ReadonlySet<string>,
  count: number,
): RecCandidate[] {
  const out: RecCandidate[] = [];
  const seen = new Set(exclude);
  const ptr = groups.map(() => 0);
  let active = true;
  while (out.length < count && active) {
    active = false;
    for (let g = 0; g < groups.length && out.length < count; g++) {
      const group = groups[g];
      while (ptr[g] < group.length) {
        const cand = group[ptr[g]++];
        const key = normalizeMeta(cand.title, cand.artist);
        if (seen.has(key)) continue;
        seen.add(key);
        out.push(cand);
        active = true;
        break;
      }
    }
  }
  return out;
}

/** Existing library row for this candidate: `itunes:<id>` identity wins,
    then normalized title+artist; null = needs resolve+persist. */
export function findLibraryMatch(
  cand: RecCandidate,
  tracks: {
    id: number;
    title: string;
    artist: string;
    mbRecordingId?: string | null;
  }[],
): number | null {
  const identity = `itunes:${cand.itunesId}`;
  const byId = tracks.find((t) => t.mbRecordingId === identity);
  if (byId) return byId.id;
  const key = normalizeMeta(cand.title, cand.artist);
  const byMeta = tracks.find(
    (t) => normalizeMeta(t.title, t.artist) === key,
  );
  return byMeta?.id ?? null;
}
