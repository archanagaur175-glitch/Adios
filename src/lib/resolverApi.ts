import { invoke } from "@tauri-apps/api/core";
import { inTauri, isAndroid } from "@/lib/api";

/** Resolver commands (P4.4): chain resolution, health, instance testing,
    and the yt-dlp updater. */

/** Resolved playable source returned by the chain (Rust `Candidate`). */
export interface Candidate {
  urlOrPath: string;
  mime: string | null;
  expiresAt: string | null;
  source: string;
  confidence: number;
}

export interface ResolverHealth {
  name: string;
  status: "ok" | "degraded" | "down";
  latencyMs: number;
}

export interface InstanceHealth {
  ok: boolean;
  flavor: string | null;
  latencyMs: number;
  message: string;
}

/** Run one query through the resolver chain (remote hits get write-back). */
export async function resolveQuery(query: {
  artist: string;
  title: string;
  album?: string | null;
  durationMs?: number | null;
}): Promise<Candidate | null> {
  if (!inTauri) return null;
  return invoke<Candidate>("resolve_query", {
    artist: query.artist,
    title: query.title,
    album: query.album ?? null,
    durationMs: query.durationMs ?? null,
  });
}

/** P19.3/D33: Android resolve via the Kotlin yt-dlp plugin — same
    Candidate shape as the Rust chain so callers stay platform-agnostic. */
export async function resolveAndroid(query: {
  artist: string;
  title: string;
  album?: string | null;
  durationMs?: number | null;
}): Promise<Candidate | null> {
  if (!inTauri) return null;
  return invoke<Candidate>("resolve_android", {
    artist: query.artist,
    title: query.title,
    album: query.album ?? null,
    durationMs: query.durationMs ?? null,
  });
}

/** P19.3: single routing choke point — Android → Kotlin plugin, every
    other platform → proven Rust chain. Lives here (not in onlinePlay) so
    prime/inflight/batch-fallback all share it and desktop behavior is
    byte-identical (isAndroid() is false there). */
export async function resolveOnline(query: {
  artist: string;
  title: string;
  album?: string | null;
  durationMs?: number | null;
}): Promise<Candidate | null> {
  return isAndroid() ? resolveAndroid(query) : resolveQuery(query);
}

/** One slot of a `resolve_batch` response (index-aligned with the request). */
export interface BatchItem {
  ok: boolean;
  candidate: Candidate | null;
  error: string | null;
}

/** P20/D34: chunk outcome — `latched` means yt-dlp could not START, so the
    FE stops batching for the session and every pick falls back to the chain. */
export interface BatchOutcome {
  latched: boolean;
  items: BatchItem[];
}

/** P20.2: ONE yt-dlp process for up to 8 queries (import save engine). */
export async function resolveBatch(
  queries: Array<{
    artist: string;
    title: string;
    album?: string | null;
    durationMs?: number | null;
  }>,
): Promise<BatchOutcome> {
  if (!inTauri) {
    return {
      latched: false,
      items: queries.map(() => ({ ok: false, candidate: null, error: "not in Tauri" })),
    };
  }
  return invoke<BatchOutcome>("resolve_batch", {
    queries: queries.map((q) => ({
      artist: q.artist,
      title: q.title,
      album: q.album ?? null,
      durationMs: q.durationMs ?? null,
    })),
  });
}

/** Per-resolver health sweep (Settings diagnostics). */
export async function resolverHealth(): Promise<ResolverHealth[]> {
  if (!inTauri) return [];
  return invoke<ResolverHealth[]>("resolver_health");
}

/** Probe one Piped/Invidious instance; never throws — reports ok: false. */
export async function testInstance(url: string): Promise<InstanceHealth> {
  if (!inTauri) {
    return { ok: false, flavor: null, latencyMs: 0, message: "not in Tauri" };
  }
  return invoke<InstanceHealth>("test_instance", { url });
}

/** Download + verify the latest yt-dlp build; resolves to its version. */
export async function updateYtdlp(): Promise<string> {
  return invoke<string>("update_ytdlp");
}
