/** P9.7: pure identity helpers for online search picks — kept import-free
    (no tauri/zustand/window) so they are unit-testable in node. */

export interface OnlinePick {
  /** MusicBrainz recording id (MB fallback rows). */
  mbid?: string;
  /** iTunes track id (primary rows); either id dedupes the DB row. */
  itunesId?: number;
  title: string;
  artist?: string;
  durationMs?: number;
}

/** Stable cache key: iTunes id wins, then MB id, then metadata text. */
export function pickKey(pick: OnlinePick): string {
  if (pick.itunesId != null) return `itunes:${pick.itunesId}`;
  if (pick.mbid && !pick.mbid.startsWith("itunes:")) return `mb:${pick.mbid}`;
  return `meta:${pick.title}|${pick.artist ?? ""}`;
}

/** DB identity string stored in tracks.mb_recording_id. */
export function identityId(pick: OnlinePick): string {
  return pick.itunesId != null ? `itunes:${pick.itunesId}` : (pick.mbid ?? "");
}
