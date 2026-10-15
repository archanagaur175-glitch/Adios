import { convertFileSrc } from "@tauri-apps/api/core";
import { inTauri } from "@/lib/api";
import { formatDuration } from "@/lib/format";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { IconMute, IconNext, IconPause, IconPlay, IconPrev, IconVolume } from "./icons";
import "./playerbar.css";

/** Fixed bottom player bar: transport, seek, volume (P3.3). */
export function PlayerBar() {
  const currentTrackId = usePlayerStore((s) => s.currentTrackId);
  const isPlaying = usePlayerStore((s) => s.isPlaying);
  const positionSec = usePlayerStore((s) => s.positionSec);
  const durationSec = usePlayerStore((s) => s.durationSec);
  const volume = usePlayerStore((s) => s.volume);
  const muted = usePlayerStore((s) => s.muted);
  const hasTrack = usePlayerStore((s) => s.queue.length > 0 && s.currentTrackId != null);
  const track = useLibraryStore((s) =>
    s.tracks.find((t) => t.id === currentTrackId) ?? null,
  );

  const toggle = () => usePlayerStore.getState().setPlaying(!isPlaying);
  const seekPosition = Math.min(positionSec, durationSec || positionSec);
  const cover =
    inTauri && track?.coverPath ? convertFileSrc(track.coverPath) : null;

  return (
    <footer className="player-bar" aria-label="Player">
      <div className="player-bar__info">
        <div className="player-bar__cover" aria-hidden="true">
          {cover ? <img src={cover} alt="" /> : <span>♪</span>}
        </div>
        <div className="player-bar__text">
          <p className="player-bar__title">{track?.title ?? "Nothing playing"}</p>
          <p className="player-bar__artist">{track?.artist ?? "—"}</p>
        </div>
      </div>

      <div className="player-bar__center">
        <div className="player-bar__buttons">
          <button
            type="button"
            aria-label="Previous track"
            disabled={!hasTrack}
            onClick={() => usePlayerStore.getState().prev()}
            className="player-bar__btn"
          >
            <IconPrev />
          </button>
          <button
            type="button"
            aria-label={isPlaying ? "Pause" : "Play"}
            disabled={!hasTrack}
            onClick={toggle}
            className="player-bar__btn player-bar__btn--primary"
          >
            {isPlaying ? <IconPause /> : <IconPlay />}
          </button>
          <button
            type="button"
            aria-label="Next track"
            disabled={!hasTrack}
            onClick={() => usePlayerStore.getState().next()}
            className="player-bar__btn"
          >
            <IconNext />
          </button>
        </div>
        <div className="player-bar__seek">
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
            onChange={(e) => usePlayerStore.getState().seekTo(Number(e.target.value))}
          />
          <span className="player-bar__time tabular">
            {formatDuration(durationSec * 1000)}
          </span>
        </div>
      </div>

      <div className="player-bar__volume">
        <button
          type="button"
          aria-label={muted ? "Unmute" : "Mute"}
          onClick={() => usePlayerStore.getState().setMuted(!muted)}
          className="player-bar__btn"
        >
          {muted || volume === 0 ? <IconMute /> : <IconVolume />}
        </button>
        <input
          type="range"
          aria-label="Volume"
          min={0}
          max={1}
          step={0.01}
          value={muted ? 0 : volume}
          onChange={(e) => usePlayerStore.getState().setVolume(Number(e.target.value))}
        />
      </div>
    </footer>
  );
}
