/** P4.4: stream-URL expiry recovery (architecture §3.3 "re-resolve once").
    When a proxied stream track's media element errors (upstream URL died
    mid-session, re-resolve here; a 403 at response start is already handled
    Rust-side by the proxy), we re-run the chain ONCE per track per session,
    patch the track URL in memory, and swap the element source back in at
    the last position. A second failure surfaces an error toast. */
import { trackMediaUrl } from "@/lib/api";
import { resolveOnline } from "@/lib/resolverApi";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";

/** Track ids already given their single automatic re-resolve this session. */
const attempted = new Set<number>();

export function hasAttemptedReResolve(trackId: number): boolean {
  return attempted.has(trackId);
}

function giveUp(title: string): void {
  useUiStore.getState().pushToast({
    message: `Stream unavailable — could not re-resolve "${title}"`,
    tone: "danger",
  });
}

/** Re-run the chain for a stream track; patch the store on success. */
async function reResolveTrack(trackId: number): Promise<boolean> {
  const library = useLibraryStore.getState();
  const track = library.tracks.find((t) => t.id === trackId);
  if (!track) return false;
  if (attempted.has(trackId)) {
    giveUp(track.title);
    return false;
  }
  attempted.add(trackId);

  let candidate;
  try {
    candidate = await resolveOnline({
      artist: track.artist,
      title: track.title,
      album: track.album,
      durationMs: track.durationMs,
    });
  } catch (error) {
    console.error("[recover] resolve_query failed:", error);
  }
  if (!candidate) {
    giveUp(track.title);
    return false;
  }
  const fresh = candidate.urlOrPath;
  library.setTracks(
    library.tracks.map((t) => (t.id === trackId ? { ...t, url: fresh, path: track.path } : t)),
  );
  console.log(`[recover] re-resolved "${track.title}" -> ${candidate.source}`);
  return true;
}

/** Called from the engine's media error hook: recover the current element.
    Returns true when a new source was set (and playback resumed if it was
    playing); false means the caller should stop playback. */
export async function recoverMediaError(
  el: HTMLAudioElement,
  deps: { isCurrent: () => boolean; resume: () => Promise<void> },
): Promise<boolean> {
  if (!deps.isCurrent()) return false;
  const trackId = usePlayerStore.getState().currentTrackId;
  if (trackId == null) return false;
  const track = useLibraryStore.getState().tracks.find((t) => t.id === trackId);
  if (!track?.url || track.path) return false; // local-file errors never re-resolve

  if (!(await reResolveTrack(trackId))) return false;

  const fresh = useLibraryStore.getState().tracks.find((t) => t.id === trackId);
  const url = fresh ? await trackMediaUrl(fresh) : null;
  if (!url) return false;

  const position = el.currentTime;
  el.addEventListener("loadedmetadata", () => {
    if (Number.isFinite(position) && position > 0) el.currentTime = position;
  }, { once: true });
  el.src = url;
  if (usePlayerStore.getState().isPlaying) await deps.resume();
  return true;
}
