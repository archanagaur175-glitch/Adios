import { useCallback, useEffect, useState } from "react";
import { useParams } from "react-router";
import {
  getPlaylist,
  removePlaylistTrack,
  renamePlaylist,
  reorderPlaylist,
} from "@/lib/playlistApi";
import { AddTracks } from "@/components/playlists/AddTracks";
import { DragTrackList } from "@/components/playlists/DragTrackList";
import { PlaylistHeaderActions } from "@/components/playlists/PlaylistHeaderActions";
import { refreshCachedIds } from "@/lib/downloadApi";
import { useDownloadStore } from "@/store/downloadStore";
import { usePlayerStore } from "@/store/playerStore";
import type { PlaylistDetail } from "@/lib/types";

/** Playlist detail (P8.2): rename/delete, add tracks, drag-reorder, export.
    P14: header buttons (Play/Shuffle/Download/Delete/Export) moved to
    PlaylistHeaderActions. */
export default function PlaylistDetailPage() {
  const { id: idParam } = useParams();
  const id = Number(idParam);
  const [detail, setDetail] = useState<PlaylistDetail | null>(null);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const cachedIds = useDownloadStore((s) => s.cachedIds);

  const refresh = useCallback(async () => {
    const next = await getPlaylist(id);
    setDetail(next);
    setName(next?.name ?? "");
    if (next) void refreshCachedIds(next.tracks.map((t) => t.id));
  }, [id]);

  useEffect(() => {
    void refresh().catch((err) => setError(String(err)));
  }, [refresh]);

  async function rename() {
    const trimmed = name.trim();
    if (!trimmed || !detail) return;
    try {
      await renamePlaylist(id, trimmed);
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  function play(trackId: number) {
    if (!detail) return;
    usePlayerStore
      .getState()
      .playQueue(detail.tracks.map((t) => t.id), detail.tracks.findIndex((t) => t.id === trackId));
  }

  async function reorder(trackIds: number[]) {
    try {
      await reorderPlaylist(id, trackIds);
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function removeTrack(trackId: number) {
    try {
      await removePlaylistTrack(id, trackId);
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  if (!detail) {
    return (
      <section className="page">
        <p className="text-ink-3">{error ?? "Loading…"}</p>
      </section>
    );
  }

  const memberIds = new Set(detail.tracks.map((t) => t.id));

  return (
    <section className="page flex h-full flex-col gap-4" data-testid="playlist-detail">
      <header className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onBlur={() => void rename()}
            onKeyDown={(e) => {
              if (e.key === "Enter") void rename();
            }}
            aria-label="Playlist name"
            className="w-72 rounded-ctrl border border-stroke bg-transparent px-3 py-2 text-20 font-display text-ink-1 focus:border-accent"
          />
        </div>
        <PlaylistHeaderActions
          playlistId={id}
          name={detail.name}
          tracks={detail.tracks}
          onChanged={() => void refresh()}
        />
      </header>

      {error && <p className="text-danger text-14">{error}</p>}
      <p className="text-12 tabular text-ink-3">
        {detail.tracks.length} track{detail.tracks.length === 1 ? "" : "s"}
      </p>

      {detail.tracks.length === 0 ? (
        <p className="text-ink-3">No tracks yet — search below to add some.</p>
      ) : (
        <div className="flex-1 overflow-y-auto">
          <DragTrackList
            tracks={detail.tracks}
            onReorder={(ids) => void reorder(ids)}
            onRemove={(trackId) => void removeTrack(trackId)}
            onPlay={play}
            cachedIds={cachedIds}
          />
        </div>
      )}

      <AddTracks playlistId={id} memberIds={memberIds} onAdded={() => void refresh()} />
    </section>
  );
}
