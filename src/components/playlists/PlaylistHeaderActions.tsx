import { useNavigate } from "react-router";
import { save } from "@tauri-apps/plugin-dialog";
import { deletePlaylist, exportPlaylistFile } from "@/lib/playlistApi";
import { downloadTracks } from "@/lib/downloadApi";
import { startRecs, stopRecs } from "@/lib/recQueue";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";
import type { Track } from "@/lib/types";

/** P14: playlist header buttons — Play, Shuffle, Download playlist, Delete,
    Export JSON. Extracted from PlaylistDetail (line budget).
    P15: Recommendations toggle — queue-only recs for this playlist. */

interface Props {
  playlistId: number;
  name: string;
  tracks: Track[];
  onChanged: () => void;
}

const btn =
  "rounded-ctrl border border-stroke px-4 py-2 text-14 text-ink-2 transition hover:bg-glass";

export function PlaylistHeaderActions({ playlistId, name, tracks, onChanged }: Props) {
  const navigate = useNavigate();
  const pushToast = useUiStore((s) => s.pushToast);
  const recOn = useUiStore((s) => s.recPrefs[playlistId] ?? false);

  function playAll() {
    if (tracks.length === 0) return;
    usePlayerStore.getState().playQueue(tracks.map((t) => t.id), 0);
    if (recOn) startRecs(playlistId, tracks);
  }

  function shuffleAll() {
    if (tracks.length === 0) return;
    const ids = tracks.map((t) => t.id);
    for (let i = ids.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [ids[i], ids[j]] = [ids[j], ids[i]];
    }
    usePlayerStore.getState().playQueue(ids, 0);
    if (recOn) startRecs(playlistId, tracks);
  }

  /** P15: ON starts immediately (if something is queued); OFF aborts the
      session and purges queued recs at once. */
  function toggleRecs() {
    const next = !recOn;
    useUiStore.getState().setRecPref(playlistId, next);
    if (next) startRecs(playlistId, tracks);
    else stopRecs(playlistId);
  }

  async function remove() {
    try {
      await deletePlaylist(playlistId);
      navigate("/playlists");
    } catch (err) {
      pushToast({ message: String(err), tone: "danger" });
    }
  }

  async function exportFile() {
    const path = await save({
      defaultPath: `${name.replace(/[^\w\- ]+/g, "")}.json`,
      filters: [{ name: "Adios playlist", extensions: ["json"] }],
    });
    if (typeof path !== "string") return;
    try {
      const count = await exportPlaylistFile(playlistId, path);
      pushToast({ message: `Exported ${count} track(s)`, tone: "info" });
    } catch (err) {
      pushToast({ message: String(err), tone: "danger" });
    }
  }

  /** P12: sequential pinned download of every stream member. */
  async function downloadAll() {
    const streams = tracks.filter((t) => t.source === "stream");
    if (streams.length === 0) {
      pushToast({ message: "No stream tracks to download", tone: "info" });
      return;
    }
    pushToast({ message: `Downloading ${streams.length} track(s)…`, tone: "info" });
    const done = await downloadTracks(streams.map((t) => t.id));
    pushToast({ message: `Downloaded ${done}/${streams.length} track(s)`, tone: "info" });
    onChanged();
  }

  return (
    <div className="flex gap-2">
      <button
        type="button"
        onClick={playAll}
        disabled={tracks.length === 0}
        className="rounded-ctrl bg-accent px-4 py-2 text-14 font-medium text-[color:var(--on-accent)] transition hover:opacity-90 disabled:opacity-50"
      >
        ▶ Play
      </button>
      <button
        type="button"
        onClick={shuffleAll}
        disabled={tracks.length === 0}
        className={btn}
      >
        Shuffle
      </button>
      <button
        type="button"
        onClick={toggleRecs}
        aria-pressed={recOn}
        title="Recommendations: queue similar tracks while this playlist plays (queue only — the playlist is untouched)"
        className={`${btn} min-w-40 ${recOn ? "border-accent bg-accent/15 text-accent" : ""}`}
      >
        {recOn ? "Recs: on" : "Recs: off"}
      </button>
      <button type="button" onClick={() => void downloadAll()} className={btn}>
        Download playlist
      </button>
      <button type="button" onClick={() => void remove()} className={btn}>
        Delete
      </button>
      <button
        type="button"
        onClick={() => void exportFile()}
        className={btn}
      >
        Export JSON
      </button>
    </div>
  );
}
