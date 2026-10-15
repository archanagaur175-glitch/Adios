import { useEffect, useState } from "react";
import type { ReactElement } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./titlebar.css";

const appWindow = getCurrentWindow();

type Glyph = "minimize" | "maximize" | "restore" | "close";

const glyphs: Record<Glyph, ReactElement> = {
  minimize: <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5h10" /></svg>,
  maximize: <svg viewBox="0 0 10 10" aria-hidden="true"><rect x="0.5" y="0.5" width="9" height="9" /></svg>,
  restore: (
    <svg viewBox="0 0 10 10" aria-hidden="true">
      <rect x="0.5" y="2.5" width="7" height="7" />
      <path d="M2.5 2.5V0.5h7v7h-2" />
    </svg>
  ),
  close: <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 0l10 10M10 0L0 10" /></svg>,
};

const actions: { glyph: Glyph; label: string; run: () => Promise<void> }[] = [
  { glyph: "minimize", label: "Minimize", run: () => appWindow.minimize() },
  { glyph: "maximize", label: "Maximize", run: () => appWindow.toggleMaximize() },
  { glyph: "close", label: "Close", run: () => appWindow.close() },
];

export function Titlebar() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const sync = async () => {
      if (!cancelled) setMaximized(await appWindow.isMaximized());
    };
    void sync();
    const unlisten = appWindow.onResized(() => void sync());
    return () => {
      cancelled = true;
      void unlisten.then((f) => f());
    };
  }, []);

  return (
    <header className="titlebar" data-tauri-drag-region>
      <span className="titlebar__brand" data-tauri-drag-region>
        <span className="titlebar__name" data-tauri-drag-region>
          Adios
        </span>
      </span>
      <div className="titlebar__controls">
        {actions.map(({ glyph, label, run }) => (
          <button
            key={label}
            type="button"
            aria-label={label}
            className={`titlebar__btn titlebar__btn--${glyph}`}
            onClick={() => void run()}
          >
            {glyph === "maximize" && maximized ? glyphs.restore : glyphs[glyph]}
          </button>
        ))}
      </div>
    </header>
  );
}
