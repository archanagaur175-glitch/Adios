import { useEffect } from "react";
import { formatDuration } from "@/lib/format";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";
import { runWithViewTransition } from "@/lib/viewTransition";
import {
  IconNext,
  IconPause,
  IconPlay,
  IconPrev,
  IconRepeat,
  IconRepeatOne,
  IconShuffle,
} from "@/components/player/icons";
import { AddToPlaylistButton } from "@/components/player/AddToPlaylistButton";
import type { RepeatMode } from "@/store/playerStore";
import { HeroArt } from "./HeroArt";
import { LyricsPanel } from "@/components/lyrics/LyricsPanel";
import "./nowplaying.css";
import "./nowplaying.mobile.css";

/** P6.4 expanded Now Playing (DESIGN §5/L3): hero art + meta + transport,
    synced lyrics column (P7.2) and the queue list. Opens via
    the player-bar cover (View Transition morph); Escape / × closes. */
export function NowPlaying() {
  const open = useUiStore((s) => s.nowPlayingOpen);
  const currentTrackId = usePlayerStore((s) => s.currentTrackId);
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const queue = usePlayerStore((s) => s.queue);
  const queueIndex = usePlayerStore((s) => s.queueIndex);
  const shuffle = usePlayerStore((s) => s.shuffle);
  const repeatMode = usePlayerStore((s) => s.repeatMode);
  const positionSec = usePlayerStore((s) => s.positionSec);
  const durationSec = usePlayerStore((s) => s.durationSec);
  const seekPosition = Math.min(positionSec, durationSec || positionSec);
  const repeatLabel: Record<RepeatMode, string> = {
    off: "Repeat off",
    queue: "Repeat queue",
    track: "Repeat one",
  };
  const tracks = useLibraryStore((s) => s.tracks);
  const track = tracks.find((t) => t.id === currentTrackId) ?? null;

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      // The immersive lyrics layer sits on top — ESC closes only that layer.
      if (useUiStore.getState().lyricsOpen) return;
      runWithViewTransition(() => useUiStore.getState().setNowPlayingOpen(false));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  if (!open) return null;

  const close = () =>
    runWithViewTransition(() => useUiStore.getState().setNowPlayingOpen(false));

  return (
    <div className="np" role="dialog" aria-label="Now playing">
      <button
        type="button"
        className="np__close np__chrome"
        aria-label="Close now playing"
        onClick={close}
      >
        ×
      </button>
      <div className="np__grid">
        <div className="np__hero">
          <HeroArt coverPath={track?.coverPath ?? null} />
          <div className="np__meta np__chrome">
            <h2 className="np__title">{track?.title ?? "Nothing playing"}</h2>
            <p className="np__artist">{track?.artist ?? "—"}</p>
          </div>
          <div className="np__transport np__chrome">
            <button
              type="button"
              aria-label="Shuffle"
              aria-pressed={shuffle}
              className="np__btn"
              onClick={() => usePlayerStore.getState().setShuffle(!shuffle)}
            >
              <IconShuffle />
            </button>
            <button
              type="button"
              aria-label="Previous track"
              className="np__btn"
              onClick={() => usePlayerStore.getState().prev()}
            >
              <IconPrev />
            </button>
            <button
              type="button"
              aria-label={isPlaying ? "Pause" : "Play"}
              className="np__btn np__btn--primary"
              onClick={() => usePlayerStore.getState().setPlaying(!isPlaying)}
            >
              {isPlaying ? <IconPause /> : <IconPlay />}
            </button>
            <button
              type="button"
              aria-label="Next track"
              className="np__btn"
              onClick={() => usePlayerStore.getState().next()}
            >
              <IconNext />
            </button>
            <button
              type="button"
              aria-label={repeatLabel[repeatMode]}
              aria-pressed={repeatMode !== "off"}
              className="np__btn"
              onClick={() =>
                usePlayerStore.getState().setRepeatMode(
                  repeatMode === "off"
                    ? "queue"
                    : repeatMode === "queue"
                      ? "track"
                      : "off",
                )
              }
            >
              {repeatMode === "track" ? <IconRepeatOne /> : <IconRepeat />}
            </button>
            <AddToPlaylistButton />
          </div>
          {/* seek row: PlayerBar's slider sits under this overlay (worse on
              phones, whose .np background is 0.88 opaque) — show times here.
              Reuses player-bar__seek styles (incl. 44px mobile touch). */}
          <div className="player-bar__seek np__chrome w-full max-w-96">
            <span className="player-bar__time tabular">
              {formatDuration(seekPosition * 1000)}
            </span>
            <input
              type="range"
              aria-label="Seek"
              min={0}
              max={durationSec > 0 ? durationSec : 0}
              step={0.1}
              value={seekPosition}
              disabled={durationSec <= 0}
              onChange={(e) =>
                usePlayerStore.getState().seekTo(Number(e.target.value))
              }
            />
            <span className="player-bar__time tabular">
              {formatDuration(durationSec * 1000)}
            </span>
          </div>
        </div>

        <div className="np__side np__chrome">
          <section className="np__panel">
            <h3 className="np__panel-title">Lyrics</h3>
            <LyricsPanel />
          </section>
          <section className="np__panel np__panel--queue">
            <h3 className="np__panel-title">Queue</h3>
            <ol className="np__queue">
              {queue.map((id, i) => {
                const t = tracks.find((x) => x.id === id);
                return (
                  <li key={`${id}-${i}`}>
                    <button
                      type="button"
                      className={`np__queue-item${i === queueIndex ? " np__queue-item--active" : ""}`}
                      onClick={() =>
                        usePlayerStore.getState().playQueue(queue, i)
                      }
                    >
                      {t ? `${t.title} — ${t.artist}` : `#${id}`}
                    </button>
                  </li>
                );
              })}
            </ol>
          </section>
        </div>
      </div>
    </div>
  );
}
