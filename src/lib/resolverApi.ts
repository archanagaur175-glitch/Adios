import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "@/lib/api";

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
