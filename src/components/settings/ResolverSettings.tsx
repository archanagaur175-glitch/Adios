import { useEffect, useState } from "react";
import { inTauri, isAndroid } from "@/lib/api";
import {
  resolverHealth,
  testInstance,
  type InstanceHealth,
  type ResolverHealth,
} from "@/lib/resolverApi";
import { useSettingsStore } from "@/store/settingsStore";

const ctrlBtn =
  "rounded-ctrl px-3 py-1.5 text-13 font-medium transition disabled:opacity-50";
const inputCls =
  "rounded-ctrl border border-stroke bg-glass px-3 py-1.5 text-14 text-ink-1 focus:border-accent";

/** Resolver settings (P4.4): health sweep + Piped/Invidious instance editor
    with per-instance Test. Instances ship EMPTY (D4) — user-managed only. */
export function ResolverSettings() {
  const setSettings = useSettingsStore((s) => s.set);
  const [rows, setRows] = useState<string[]>([]);
  const [health, setHealth] = useState<ResolverHealth[]>([]);
  const [results, setResults] = useState<Record<string, InstanceHealth>>({});
  const [testing, setTesting] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [savedNote, setSavedNote] = useState(false);

  useEffect(() => {
    setRows(useSettingsStore.getState().pipedInstances);
  }, []);

  function save() {
    const cleaned = rows.map((r) => r.trim().replace(/\/+$/, "")).filter(Boolean);
    setSettings({ pipedInstances: cleaned });
    setSavedNote(true);
    setTimeout(() => setSavedNote(false), 2000);
  }

  async function test(url: string) {
    setTesting(url);
    try {
      const res = await testInstance(url);
      setResults((prev) => ({ ...prev, [url]: res }));
    } finally {
      setTesting(null);
    }
  }

  async function checkAll() {
    setChecking(true);
    try {
      const res = await resolverHealth();
      // Android has no yt-dlp (D19) — the row would always read "down".
      setHealth(isAndroid() ? res.filter((h) => h.name !== "ytdlp") : res);
    } catch {
      setHealth([]);
    } finally {
      setChecking(false);
    }
  }

  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <h2 className="text-18 font-display">Resolver</h2>
        <button
          type="button"
          onClick={() => void checkAll()}
          disabled={checking || !inTauri}
          className={`${ctrlBtn} bg-accent text-[color:var(--on-accent)] hover:opacity-90`}
        >
          {checking ? "Checking…" : "Check resolvers"}
        </button>
      </div>

      {health.length > 0 && (
        <ul className="flex flex-col gap-1 text-13 text-ink-3 tabular">
          {health.map((h) => (
            <li key={h.name} className="flex justify-between">
              <span>
                {h.name} —{" "}
                <span className={h.status === "ok" ? "text-accent" : "text-danger"}>
                  {h.status}
                </span>
              </span>
              <span>{h.latencyMs} ms</span>
            </li>
          ))}
        </ul>
      )}

      <p className="text-13 text-ink-3">
        Piped / Invidious instances (one URL per line, empty = fallback off).
      </p>
      <div className="flex flex-col gap-2">
        {rows.map((row, i) => {
          const result = results[row.trim()];
          return (
            <div key={i} className="flex flex-col gap-1">
              <div className="flex gap-2">
                <input
                  value={row}
                  onChange={(e) =>
                    setRows(rows.map((r, idx) => (idx === i ? e.target.value : r)))
                  }
                  placeholder="https://example.instance"
                  className={`${inputCls} min-w-0 flex-1`}
                />
                <button
                  type="button"
                  onClick={() => void test(row.trim())}
                  disabled={testing !== null || !row.trim() || !inTauri}
                  className={`${ctrlBtn} border border-stroke text-ink-1 hover:bg-glass`}
                >
                  {testing === row.trim() ? "…" : "Test"}
                </button>
                <button
                  type="button"
                  onClick={() => setRows(rows.filter((_, idx) => idx !== i))}
                  className={`${ctrlBtn} border border-stroke text-ink-1 hover:bg-glass`}
                  aria-label="Remove instance"
                >
                  ✕
                </button>
              </div>
              {result && (
                <p className={result.ok ? "text-12 text-accent" : "text-12 text-danger"}>
                  {result.ok
                    ? `${result.flavor} · ${result.latencyMs} ms`
                    : result.message}
                </p>
              )}
            </div>
          );
        })}
      </div>

      <div className="flex gap-2">
        <button
          type="button"
          onClick={() => setRows([...rows, ""])}
          className={`${ctrlBtn} border border-stroke text-ink-1 hover:bg-glass`}
        >
          Add instance
        </button>
        <button
          type="button"
          onClick={save}
          disabled={!inTauri}
          className={`${ctrlBtn} bg-accent text-[color:var(--on-accent)] hover:opacity-90`}
        >
          {savedNote ? "Saved ✓" : "Save"}
        </button>
      </div>
    </section>
  );
}
