import { listen } from "@tauri-apps/api/event";
import { inTauri } from "@/lib/api";
import { useSettingsStore } from "@/store/settingsStore";

/** OQ2/P9.4: app-lifetime listener keeping `ytdlpPath` in sync with Rust's
    hot-swap. The Settings page can unmount mid-update — its component
    listener dies with it, so the store sync lives here (single writer). */
export function initYtdlpPathSync(): () => void {
  if (!inTauri) return () => {};
  let unlisten: (() => void) | undefined;
  void listen<{ stage: string; path?: string }>("ytdlp-progress", (event) => {
    if (event.payload.stage === "done" && event.payload.path) {
      useSettingsStore.getState().set({ ytdlpPath: event.payload.path });
    }
  }).then((fn) => {
    unlisten = fn;
  });
  return () => unlisten?.();
}
