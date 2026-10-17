import { searchItunes } from "@/lib/onlineApi";
import { persistPick, resolveOnlinePick, type OnlinePick } from "@/lib/onlinePlay";
import {
  findLibraryMatch,
  normalizeMeta,
  planPicks,
  recCountFor,
  sampleArtists,
  type RecCandidate,
} from "@/lib/recommend";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";
import type { Track } from "@/lib/types";

/** P15: queue-only recommendation sessions — trickle recs into the CURRENT
    queue (playlist DB untouched; recs become normal stream rows). One active
    session at a time; `generation` invalidates an aborted loop at its next
    checkpoint, and a queue-ref guard aborts when anything but this session
    replaces the queue (a foreign playQueue passes a different array — the
    NowPlaying panel's re-click passes the same array, so it doesn't kill us). */

interface RecSession {
  gen: number;
  playlistId: number;
  recIds: Set<number>;
  queueRef: number[];
}

let generation = 0;
let active: RecSession | null = null;

function stale(session: RecSession): boolean {
  if (session.gen !== generation || active !== session) return true;
  return usePlayerStore.getState().queue !== session.queueRef;
}

function finish(session: RecSession): void {
  if (active === session) active = null;
}

/** Abort the session for `playlistId` (undefined = whichever is active) and
    purge its recs from the queue — the current track survives. */
export function stopRecs(playlistId?: number): void {
  const session = active;
  if (!session) return;
  if (playlistId != null && session.playlistId !== playlistId) return;
  generation++;
  finish(session);
  const player = usePlayerStore.getState();
  // Queue already replaced (foreign play) → our recs are gone with it.
  if (session.recIds.size > 0 && player.queue === session.queueRef) {
    player.removeQueueTracks([...session.recIds], player.currentTrackId);
  }
}

/** Start (or restart) a trickle for this playlist's tracks. Immediate even
    mid-play; no-op on an empty playlist. */
export function startRecs(playlistId: number, tracks: Track[]): void {
  // Empty queue → nothing to inject; the next Play (header/row, pref on)
  // calls startRecs again with a populated queue.
  if (recCountFor(tracks.length) <= 0) return;
  if (usePlayerStore.getState().queue.length === 0) return;
  generation++;
  const session: RecSession = {
    gen: generation,
    playlistId,
    recIds: new Set(),
    queueRef: usePlayerStore.getState().queue,
  };
  active = session;
  void run(session, tracks);
}

function toPick(c: RecCandidate): OnlinePick {
  return {
    itunesId: c.itunesId,
    title: c.title,
    artist: c.artist,
    durationMs: c.durationMs ?? undefined,
    artworkUrl: c.artworkUrl ?? undefined,
    genre: c.genre ?? undefined,
  };
}

async function run(session: RecSession, tracks: Track[]): Promise<void> {
  const seeds = sampleArtists(tracks, 3);
  const groups: RecCandidate[][] = [];
  for (const seed of seeds) {
    if (stale(session)) return finish(session);
    try {
      groups.push(await searchItunes(seed));
    } catch {
      // failed seed → fewer candidate groups; keep going
    }
  }
  if (stale(session)) return finish(session);
  const exclude = new Set(tracks.map((t) => normalizeMeta(t.title, t.artist)));
  const picks = planPicks(groups, exclude, recCountFor(tracks.length));
  let added = 0;
  for (const pick of picks) {
    if (stale(session)) return finish(session);
    const trackId = await resolve(session, pick);
    if (trackId == null) continue;
    if (stale(session)) return finish(session);
    const player = usePlayerStore.getState();
    const after = Math.max(player.queueIndex, -1) + 1;
    const span = player.queue.length - after + 1;
    player.insertQueueAt(trackId, after + Math.floor(Math.random() * span));
    session.queueRef = usePlayerStore.getState().queue;
    session.recIds.add(trackId);
    added++;
  }
  if (stale(session)) return finish(session);
  // Success: KEEP the session (recIds + queueRef) so a later toggle OFF can
  // still purge what we queued. Only abort paths clear it.
  if (added > 0) {
    const s = added === 1 ? "" : "s";
    useUiStore.getState().pushToast({
      message: `Added ${added} recommendation${s} to the queue`,
      tone: "info",
    });
  }
}

/** Fast path: already in the library → its id; else resolve + persist.
    Returns null on a miss (honest failure — skip this pick). */
async function resolve(session: RecSession, pick: RecCandidate): Promise<number | null> {
  const fast = findLibraryMatch(pick, useLibraryStore.getState().tracks);
  if (fast != null) return fast;
  try {
    const candidate = await resolveOnlinePick(toPick(pick));
    if (stale(session)) return null;
    if (!candidate) return null;
    const row = await persistPick(toPick(pick), candidate);
    return row.id;
  } catch (err) {
    console.warn("[recQueue] resolve failed:", err);
    return null;
  }
}
