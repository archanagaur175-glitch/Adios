import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { inTauri } from "@/lib/api";
import { updateYtdlp } from "@/lib/resolverApi";
import { useSettingsStore } from "@/store/settingsStore";

const ctrlBtn =
  "rounded-ctrl bg-accent px-3 py-1.5 text-13 font-medium text-[color:var(--on-accent)] transition hover:opacity-90 disabled:opacity-50";

interface Progress {
  stage: string;
  written: number;
  total: number;
  version?: string;
  path?: string;
}

/** yt-dlp updater (architecture §3.3 "Update yt-dlp"): downloads the latest
    official build with progress events, verifies it, then hot-swaps. */
export function YtdlpSettings() {
  const ytdlpPath = useSettingsStore((s) => s.ytdlpPath);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [version, setVersion] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    // UI-only progress/version display. The store's ytdlpPath is owned by
    // the app-lifetime listener in lib/ytdlpSync.ts (OQ2: survives unmount).
    let unlisten: (() => void) | undefined;
    void listen<Progress>("ytdlp-progress", (event) => {
      setProgress(event.payload);
      if (event.payload.stage === "done" && event.payload.version) {
        setVersion(event.payload.version);
        setBusy(false);
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  async function run() {
    setBusy(true);
    setError(null);
    setVersion(null);
    setProgress({ stage: "downloading", written: 0, total: 0 });
    try {
      setVersion(await updateYtdlp());
      setProgress({ stage: "done", written: 0, total: 0 });
    } catch (err) {
      setError(String(err));
      setProgress(null);
    } finally {
      setBusy(false);
    }
  }

  const pct =
    progress && progress.stage === "downloading" && progress.total > 0
      ? Math.min(100, Math.round((progress.written / progress.total) * 100))
      : null;

  return (
    <section className="flex flex-col gap-3 border-t border-stroke pt-4">
      <h2 className="text-18 font-display">yt-dlp</h2>
      <p className="text-13 text-ink-3">
        Binary: <span className="tabular text-ink-2">{ytdlpPath}</span>
        {version && <span className="text-ink-2"> · updated: {version}</span>}
      </p>
      {pct !== null && (
        <p className="text-13 tabular text-ink-3">
          Downloading… {pct}% ({progress?.written ?? 0} / {progress?.total ?? 0} bytes)
        </p>
      )}
      {error && <p className="text-13 text-danger">{error}</p>}
      <div>
        <button
          type="button"
          onClick={() => void run()}
          disabled={busy || !inTauri}
          className={ctrlBtn}
        >
          {busy ? "Updating…" : "Update yt-dlp"}
        </button>
      </div>
    </section>
  );
}
