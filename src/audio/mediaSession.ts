/** Media Session metadata/handlers + global media-key events (P3.4).
    Rust `win/media_keys.rs` emits `media-key` ("play-pause"|"next"|
    "prev"|"stop") when the OS media keys are pressed. */
import { listen } from "@tauri-apps/api/event";
import { convertFileSrc } from "@tauri-apps/api/core";
import { inTauri } from "@/lib/api";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";

const SEEK_STEP_SEC = 5;

type ActionDetails = MediaSessionActionDetails | undefined;

function applyAction(action: string, details?: ActionDetails): void {
  const store = usePlayerStore.getState();
  switch (action) {
    case "play":
      store.setPlaying(true);
      break;
    case "pause":
    case "stop":
    case "play-pause":
      if (action === "play-pause") store.setPlaying(!store.isPlaying);
      else store.setPlaying(false);
      break;
    case "previoustrack":
    case "prev":
      store.prev();
      break;
    case "nexttrack":
    case "next":
      store.next();
      break;
    case "seekto":
      if (details?.seekTime != null) store.seekTo(details.seekTime);
      break;
    case "seekbackward":
      store.seekTo(store.positionSec - SEEK_STEP_SEC);
      break;
    case "seekforward":
      store.seekTo(store.positionSec + SEEK_STEP_SEC);
      break;
  }
}

function updateMetadata(): void {
  if (!("mediaSession" in navigator)) return;
  const { currentTrackId } = usePlayerStore.getState();
  const track = useLibraryStore
    .getState()
    .tracks.find((t) => t.id === currentTrackId);
  if (!track) {
    navigator.mediaSession.metadata = null;
    return;
  }
  const artwork =
    inTauri && track.coverPath
      ? [
          {
            src: convertFileSrc(track.coverPath),
            sizes: "any",
            type: track.coverPath.toLowerCase().endsWith(".png")
              ? "image/png"
              : "image/jpeg",
          },
        ]
      : [];
  navigator.mediaSession.metadata = new MediaMetadata({
    title: track.title,
    artist: track.artist,
    album: track.album ?? "",
    artwork,
  });
}

/** Wire navigator.mediaSession + the Rust media-key events. Call once (App). */
export function initMediaIntegration(): void {
  if (!inTauri || !("mediaSession" in navigator)) return;

  const setHandler = (
    action: MediaSessionAction,
    handler: (details?: ActionDetails) => void,
  ) => {
    try {
      navigator.mediaSession.setActionHandler(action, handler);
    } catch {
      // action unsupported on this WebView2 build — ignore
    }
  };
  setHandler("play", () => applyAction("play"));
  setHandler("pause", () => applyAction("pause"));
  setHandler("stop", () => applyAction("stop"));
  setHandler("previoustrack", () => applyAction("previoustrack"));
  setHandler("nexttrack", () => applyAction("nexttrack"));
  setHandler("seekto", (details) => applyAction("seekto", details));
  setHandler("seekbackward", () => applyAction("seekbackward"));
  setHandler("seekforward", () => applyAction("seekforward"));

  void listen<string>("media-key", (event) => applyAction(event.payload));

  usePlayerStore.subscribe((state, prev) => {
    if (state.currentTrackId !== prev.currentTrackId) updateMetadata();
    if (state.isPlaying !== prev.isPlaying) {
      navigator.mediaSession.playbackState = state.isPlaying
        ? "playing"
        : "paused";
    }
  });
  updateMetadata();
  navigator.mediaSession.playbackState = usePlayerStore.getState().isPlaying
    ? "playing"
    : "paused";
}
