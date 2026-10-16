import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { useSettingsStore } from "@/store/settingsStore";
import { usePlayerStore } from "@/store/playerStore";
import { useLibraryStore } from "@/store/libraryStore";
import { useUiStore } from "@/store/uiStore";
import { engine } from "@/audio/engine";
import { attachSessionPersist, flushSession, restoreSession } from "@/lib/session";

void useSettingsStore.getState().hydrate();

if (import.meta.env.DEV) {
  // Debug hooks for the CDP smoke harness (dev-only; stripped from builds).
  (window as unknown as Record<string, unknown>).__adios = {
    engine,
    player: usePlayerStore,
    library: useLibraryStore,
    ui: useUiStore,
    session: { attachSessionPersist, restoreSession, flush: flushSession },
  };
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
