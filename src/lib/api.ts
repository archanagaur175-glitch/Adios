import { invoke } from "@tauri-apps/api/core";

/** True when running inside the Tauri webview (not plain vite/preview). */
export const inTauri = "__TAURI_INTERNALS__" in window;

/** All settings: key → parsed value. Empty map outside Tauri or on error. */
export async function getSettings(): Promise<Record<string, unknown>> {
  if (!inTauri) return {};
  try {
    const raw = await invoke<Record<string, string>>("get_settings");
    const parsed: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(raw)) {
      try {
        parsed[key] = JSON.parse(value);
      } catch {
        // skip malformed rows rather than breaking startup
      }
    }
    return parsed;
  } catch (error) {
    console.error("[api] get_settings failed:", error);
    return {};
  }
}

/** Persist one setting (value is JSON-serialized on this side). */
export async function setSetting(key: string, value: unknown): Promise<void> {
  if (!inTauri) return;
  try {
    await invoke("set_setting", { key, valueJson: JSON.stringify(value) });
  } catch (error) {
    console.error(`[api] set_setting(${key}) failed:`, error);
  }
}
