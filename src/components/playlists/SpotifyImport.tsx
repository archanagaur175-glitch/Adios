import { useState } from "react";
import { isSaving, savePicksToPlaylist } from "@/lib/playlistSave";
import { fetchSpotifyPlaylist, spotifyCandidates } from "@/lib/spotifyApi";
import { useUiStore } from "@/store/uiStore";

/** P17 (D18): paste-link import of a PUBLIC Spotify playlist — anonymous
    embed-SSR fetch in Rust, then the shared sequential save engine with an
    inline `Saving n/total…` readout. Honest failure toasts only (invalid
    link / not found or private / endpoint changed). */

type Phase = "idle" | "fetching" | "saving";

export function SpotifyImport({ onImported }: { onImported: () => void }) {
  const pushToast = useUiStore((s) => s.pushToast);
  const [open, setOpen] = useState(false);
  const [url, setUrl] = useState("");
  const [phase, setPhase] = useState<Phase>("idle");
  const [saving, setSaving] = useState({ done: 0, total: 0 });

  const busy = phase !== "idle" || isSaving();

  async function run() {
    const link = url.trim();
    if (!link || busy) return;
    setPhase("fetching");
    try {
      const playlist = await fetchSpotifyPlaylist(link);
      const candidates = spotifyCandidates(playlist);
      if (candidates.length === 0) {
        pushToast({ message: `“${playlist.name}” has no importable tracks`, tone: "danger" });
        return;
      }
      setPhase("saving");
      setSaving({ done: 0, total: candidates.length });
      const res = await savePicksToPlaylist(playlist.name, candidates, (done, total) =>
        setSaving({ done, total }),
      );
      if (!res || res.added === 0) {
        pushToast({
          message: `Nothing saved for “${playlist.name}” — no streams found`,
          tone: "danger",
        });
        return;
      }
      onImported();
      setUrl("");
      setOpen(false);
      const note = res.skipped > 0 ? ` (${res.skipped} skipped)` : "";
      pushToast({
        message: `Saved ${res.added} track${res.added === 1 ? "" : "s"} to “${res.name}”${note}`,
        tone: "info",
      });
    } catch (err) {
      pushToast({ message: String(err), tone: "danger" });
    } finally {
      setPhase("idle");
      setSaving({ done: 0, total: 0 });
    }
  }

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="self-start rounded-ctrl border border-stroke px-4 py-2 text-14 text-ink-2 transition hover:bg-glass"
      >
        Import from Spotify
      </button>
    );
  }
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-2">
        <input
          type="text"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void run();
          }}
          placeholder="Paste a Spotify playlist link…"
          aria-label="Spotify playlist link"
          disabled={busy}
          className="w-96 rounded-ctrl border border-stroke bg-transparent px-3 py-2 text-14 text-ink-1 placeholder:text-ink-3 focus:border-accent disabled:opacity-60"
        />
        <button
          type="button"
          onClick={() => void run()}
          disabled={busy || !url.trim()}
          className="rounded-ctrl bg-accent px-4 py-2 text-14 font-medium text-[color:var(--on-accent)] transition hover:opacity-90 disabled:opacity-50"
        >
          {phase === "fetching"
            ? "Fetching…"
            : phase === "saving"
              ? `Saving ${saving.done}/${saving.total}…`
              : "Import"}
        </button>
        <button
          type="button"
          aria-label="Close Spotify import"
          onClick={() => setOpen(false)}
          disabled={busy}
          className="rounded px-2 py-1 text-14 text-ink-3 transition hover:text-ink-1 disabled:opacity-50"
        >
          ×
        </button>
      </div>
      <p className="text-12 text-ink-3">
        Public playlists only — no Spotify account or Premium needed.
      </p>
    </div>
  );
}
