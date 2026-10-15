import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { useSettingsStore } from "@/store/settingsStore";
import { usePlayerStore } from "@/store/playerStore";
import { useLibraryStore } from "@/store/libraryStore";
import { engine } from "@/audio/engine";

void useSettingsStore.getState().hydrate();

if (import.meta.env.DEV) {
  // Debug hooks for the CDP smoke harness (dev-only; stripped from builds).
  (window as unknown as Record<string, unknown>).__adios = {
    engine,
    player: usePlayerStore,
    library: useLibraryStore,
  };
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
