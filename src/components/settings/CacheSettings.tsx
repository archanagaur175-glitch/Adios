import { useEffect, useState } from "react";
import { runCacheEviction, setSetting } from "@/lib/api";
import type { EvictStats } from "@/lib/types";
import { useSettingsStore } from "@/store/settingsStore";

const inputCls =
  "w-28 rounded-ctrl border border-stroke bg-glass px-3 py-1.5 text-14 tabular text-ink-1 focus:border-accent";
const ctrlBtn =
  "rounded-ctrl bg-accent px-3 py-1.5 text-13 font-medium text-white transition hover:opacity-90 disabled:opacity-50";

const MIN_MB = 16;
const MAX_MB = 100_000;

function mb(bytes: number): string {
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

/** Cache size cap (P8.1): persists `cacheSizeMb` and applies it through
    the eviction command; shows what one pass reclaimed. */
export function CacheSettings() {
  const cacheSizeMb = useSettingsStore((s) => s.cacheSizeMb);
  const [text, setText] = useState(String(cacheSizeMb));
  const [stats, setStats] = useState<EvictStats | null>(null);
  const [busy, setBusy] = useState(false);

  async function clean() {
    setBusy(true);
    try {
      setStats(await runCacheEviction());
    } finally {
      setBusy(false);
    }
  }

  // Initial pass: report usage (and enforce the cap from a prior session).
  useEffect(() => {
    void runCacheEviction().then(setStats);
  }, []);

  // Follow startup hydration (set only when the STORE value changes).
  useEffect(() => {
    setText(String(cacheSizeMb));
  }, [cacheSizeMb]);

  function commit() {
    void (async () => {
      const n = Math.min(MAX_MB, Math.max(MIN_MB, Math.floor(Number(text) || MIN_MB)));
      setText(String(n));
      if (n !== cacheSizeMb) {
        // Write the setting FIRST (durable), then enforce it — the eviction
        // command reads the cap from SQLite, so ordering matters.
        useSettingsStore.setState({ cacheSizeMb: n });
        await setSetting("cacheSizeMb", n);
        await clean();
      }
    })();
  }

  return (
    <section className="flex flex-col gap-3 border-t border-stroke pt-4">
      <h2 className="text-18 font-display">Cache</h2>
      <div className="flex items-center gap-2">
        <label htmlFor="cache-size" className="w-36 text-13 text-ink-2">
          Size limit (MB)
        </label>
        <input
          id="cache-size"
          type="number"
          min={MIN_MB}
          max={MAX_MB}
          step={64}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
          className={inputCls}
        />
      </div>
      <div className="flex items-center gap-3">
        <button
          type="button"
          onClick={() => void clean()}
          disabled={busy || stats === null}
          className={ctrlBtn}
        >
          {busy ? "Working…" : "Clean up now"}
        </button>
        {stats !== null && (
          <span className="text-13 tabular text-ink-3">
            Using {mb(stats.remainingBytes)} of {cacheSizeMb} MB
            {stats.removed > 0 &&
              ` · removed ${stats.removed} file${stats.removed === 1 ? "" : "s"}`}
            {stats.removedParts > 0 && ` · ${stats.removedParts} temp`}
            {stats.removedOrphans > 0 && ` · ${stats.removedOrphans} orphan`}
          </span>
        )}
      </div>
      <p className="text-12 text-ink-3">
        Remote streams are cached on disk; oldest files are evicted first when
        over the limit.
      </p>
    </section>
  );
}
