import { pickKey, type OnlinePick } from "@/lib/onlinePick";
import {
  cacheCandidate,
  getCachedCandidate,
  hasInflight,
  reserveFlight,
  runChain,
} from "@/lib/onlinePlay";
import { resolveBatch } from "@/lib/resolverApi";

/** P20.2 (D34): batched prefetch for the save engine — deduped misses are
    split into `BATCH_SIZE` chunks, ≤ `MAX_ACTIVE_CHUNKS` dispatched at once,
    each dispatched key reserved as a P9.9 flight (`batch hit ?? single
    chain`), hits seeding `specCache`. A `latched` outcome (or a rejected
    invoke) trips the SESSION latch: queued chunks never dispatch and every
    remaining pick resolves through today's per-pick chain — Android and
    yt-dlp-less desktops behave exactly as before, one wasted chunk attempt
    at most. Fire-and-forget: the save loop keeps calling `resolveOnlinePick`
    unchanged (prime/inflight/cache wins are what the pump produces). */

export const BATCH_SIZE = 8;
const MAX_ACTIVE_CHUNKS = 3;

let batchLatched = false;

export function isBatchLatched(): boolean {
  return batchLatched;
}

/** Kick the prefetcher over the picks that still need resolving. */
export function resolveManyPicks(picks: OnlinePick[]): void {
  if (batchLatched || picks.length === 0) return;

  // Dedupe by identity; skip anything already primed or in flight.
  const byKey = new Map<string, OnlinePick>();
  for (const pick of picks) {
    const key = pickKey(pick);
    if (byKey.has(key) || getCachedCandidate(key) || hasInflight(key)) continue;
    byKey.set(key, pick);
  }
  const batchable = [...byKey.values()];
  if (batchable.length === 0) return;

  const queue: OnlinePick[][] = [];
  for (let i = 0; i < batchable.length; i += BATCH_SIZE) {
    queue.push(batchable.slice(i, i + BATCH_SIZE));
  }

  let active = 0;
  const pump = (): void => {
    while (active < MAX_ACTIVE_CHUNKS && queue.length > 0 && !batchLatched) {
      const chunk = queue.shift()!;
      active++;
      const work = resolveBatch(
        chunk.map((p) => ({
          artist: p.artist ?? "",
          title: p.title,
          album: null,
          durationMs: p.durationMs ?? null,
        })),
      )
        .then((outcome) => {
          if (outcome.latched) {
            batchLatched = true;
            console.warn("[batchResolve] yt-dlp spawn failed — batch latch engaged");
          }
          chunk.forEach((p, i) => {
            const item = outcome.items[i];
            if (item?.ok && item.candidate) cacheCandidate(pickKey(p), item.candidate);
          });
        })
        .catch((err) => {
          batchLatched = true;
          console.warn("[batchResolve] resolve_batch failed — latch engaged:", err);
        });
      for (const p of chunk) {
        const key = pickKey(p);
        // A chain already running for this key wins — never overwrite it.
        if (hasInflight(key)) continue;
        reserveFlight(key, work.then(() => getCachedCandidate(key) ?? runChain(p)));
      }
      void work.finally(() => {
        active--;
        pump();
      });
    }
  };
  pump();
}
