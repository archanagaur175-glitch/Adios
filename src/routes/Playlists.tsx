import { useCallback, useEffect, useRef, useState } from "react";
import { Link } from "react-router";
import { open } from "@tauri-apps/plugin-dialog";
import { inTauri } from "@/lib/api";
import { createPlaylist, deletePlaylist, importPlaylistFile } from "@/lib/playlistApi";
import { SpotifyImport } from "@/components/playlists/SpotifyImport";
import { usePlaylistStore } from "@/store/playlistStore";
import { useUiStore } from "@/store/uiStore";

/** Playlists index (P8.2): create, open, delete, import-from-JSON. */
export default function PlaylistsPage() {
  const playlists = usePlaylistStore((s) => s.playlists);
  const load = usePlaylistStore((s) => s.load);
  const pushToast = useUiStore((s) => s.pushToast);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const nameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    void load();
  }, [load]);

  const create = useCallback(async () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    setError(null);
    try {
      const id = await createPlaylist(trimmed);
      setName("");
      await load();
      if (id > 0) pushToast({ message: `Created “${trimmed}”`, tone: "info" });
    } catch (err) {
      setError(String(err));
    }
  }, [name, load, pushToast]);

  async function remove(id: number, label: string) {
    setError(null);
    try {
      await deletePlaylist(id);
      await load();
      pushToast({ message: `Deleted “${label}”`, tone: "info" });
    } catch (err) {
      setError(String(err));
    }
  }

  async function importFile() {
    setError(null);
    const path = await open({
      multiple: false,
      filters: [{ name: "Adios playlist", extensions: ["json"] }],
    });
    if (typeof path !== "string") return;
    try {
      const result = await importPlaylistFile(path);
      await load();
      if (result) {
        pushToast({
          message: `Imported ${result.added} track(s)${result.missing ? `, ${result.missing} missing` : ""}`,
          tone: "info",
        });
      }
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <section className="page flex h-full flex-col gap-4">
      <header className="flex items-center justify-between">
        <h1 className="text-28 font-display">Playlists</h1>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => void importFile()}
            disabled={!inTauri}
            className="rounded-ctrl border border-stroke px-4 py-2 text-14 text-ink-2 transition hover:bg-glass disabled:opacity-50"
          >
            Import JSON
          </button>
          <button
            type="button"
            onClick={() => {
              // P9.6 fix #3: the button used to stay greyed out until a name
              // was typed (input is below — easy to miss). Now it always
              // works: empty name → focus the input with a hint.
              if (!name.trim()) {
                nameRef.current?.focus();
                pushToast({ message: "Type a name for the playlist first", tone: "info" });
                return;
              }
              void create();
            }}
            disabled={!inTauri}
            className="rounded-ctrl bg-accent px-4 py-2 text-14 font-medium text-[color:var(--on-accent)] transition hover:opacity-90 disabled:opacity-50"
          >
            New playlist
          </button>
        </div>
      </header>

      <div className="flex gap-2">
        <input
          ref={nameRef}
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void create();
          }}
          placeholder="Playlist name"
          aria-label="Playlist name"
          className="w-72 rounded-ctrl border border-stroke bg-transparent px-3 py-2 text-14 text-ink-1 placeholder:text-ink-3 focus:border-accent"
        />
      </div>

      <SpotifyImport onImported={() => void load()} />

      {error && <p className="text-danger text-14">{error}</p>}

      {playlists.length === 0 ? (
        <p className="text-ink-3">
          {inTauri
            ? "No playlists yet — create one or import a JSON export."
            : "Playlists are available inside the Adios app."}
        </p>
      ) : (
        <ul className="flex flex-col gap-1 overflow-y-auto">
          {playlists.map((playlist) => (
            <li
              key={playlist.id}
              className="grid grid-cols-[1fr_auto_auto] items-center gap-3 rounded-card px-3 py-2 transition hover:bg-glass"
            >
              <Link
                to={`/playlists/${playlist.id}`}
                className="min-w-0 truncate text-14 text-ink-1"
              >
                {playlist.name}
              </Link>
              <span className="text-12 tabular text-ink-3">
                {playlist.trackCount} track{playlist.trackCount === 1 ? "" : "s"}
              </span>
              <button
                type="button"
                aria-label={`Delete ${playlist.name}`}
                onClick={() => void remove(playlist.id, playlist.name)}
                className="rounded px-2 py-1 text-14 text-ink-3 transition hover:text-danger"
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
