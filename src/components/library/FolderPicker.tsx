import { useCallback, useEffect, useState } from "react";
import { listDirectory, type DirListing } from "@/lib/api";

/** P18.1c: in-app folder picker — Android has no native folder dialog, so
    scanning falls back here. Path-based by design: the chosen path goes
    straight to `scan_folder` (std::fs walk, no SAF content:// URIs). */
interface Props {
  start: string;
  onPick: (path: string) => void;
  onCancel: () => void;
}

function joinPath(base: string, name: string): string {
  const sep = base.includes("\\") && !base.includes("/") ? "\\" : "/";
  return base.replace(/[\\/]+$/, "") + sep + name;
}

export function FolderPicker({ start, onPick, onCancel }: Props) {
  const [cwd, setCwd] = useState(start);
  const [listing, setListing] = useState<DirListing | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const load = useCallback(async (path: string) => {
    setErr(null);
    setListing(null);
    try {
      const next = await listDirectory(path);
      setListing(next);
      setCwd(path);
    } catch (e) {
      setErr(String(e));
    }
  }, []);

  useEffect(() => {
    void load(start);
  }, [load, start]);

  const parentPath = listing?.parent ?? null;

  return (
    <div
      className="fixed inset-0 z-40 flex justify-center bg-black/45 pt-[10vh]"
      onClick={onCancel}
      role="presentation"
    >
      <div
        className="glass relative flex h-[min(70vh,640px)] w-[min(480px,92vw)] flex-col overflow-hidden rounded-card"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Choose a folder"
      >
        <header className="flex items-center justify-between gap-3 border-b border-stroke px-4 py-3">
          <span className="truncate text-13 text-ink-2">{cwd}</span>
          <button
            type="button"
            onClick={onCancel}
            className="text-ink-3 transition hover:text-ink-1"
          >
            Cancel
          </button>
        </header>

        <div className="flex-1 overflow-y-auto px-2 py-2">
          {err && <p className="px-2 py-1 text-13 text-danger">{err}</p>}
          {parentPath && (
            <button
              type="button"
              onClick={() => void load(parentPath)}
              className="grid w-full cursor-pointer grid-cols-[auto_1fr] items-center gap-3 rounded-card px-3 py-2 text-left text-14 text-ink-2 transition hover:bg-glass"
            >
              <span aria-hidden>↑</span>
              <span>..</span>
            </button>
          )}
          {listing?.dirs.map((d) => (
            <button
              key={d}
              type="button"
              onClick={() => void load(joinPath(cwd, d))}
              className="grid w-full cursor-pointer grid-cols-[auto_1fr] items-center gap-3 rounded-card px-3 py-2 text-left text-14 text-ink-1 transition hover:bg-glass"
            >
              <span aria-hidden>▸</span>
              <span className="truncate">{d}</span>
            </button>
          ))}
          {listing && !err && listing.dirs.length === 0 && (
            <p className="px-3 py-1.5 text-13 text-ink-3">
              No sub-folders here.
            </p>
          )}
        </div>

        <footer className="flex items-center justify-between gap-3 border-t border-stroke px-4 py-3">
          <span className="truncate text-12 text-ink-3">
            Pick a folder, then scan it
          </span>
          <button
            type="button"
            onClick={() => onPick(cwd)}
            disabled={!listing}
            className="rounded-ctrl bg-accent px-4 py-2 text-14 font-medium text-[color:var(--on-accent)] transition hover:opacity-90 disabled:opacity-50"
          >
            Scan this folder
          </button>
        </footer>
      </div>
    </div>
  );
}
