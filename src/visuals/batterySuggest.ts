/** P6.3 battery auto-suggest (DESIGN §7): one toast per session when
    discharging below 20 % and Eco is not active — the user still pins the
    tier themselves (Settings → Appearance). Uses the Chromium Battery
    Status API when WebView2 exposes it; absent → silent no-op.
    `debugSuggest` is the dev/gate hook (no real battery needed). */

import { useUiStore } from "@/store/uiStore";

export interface BatteryLike {
  level: number;
  charging: boolean;
  addEventListener(type: string, listener: () => void): void;
}

type NavigatorBattery = Navigator & { getBattery?: () => Promise<BatteryLike> };

let suggested = false;

function maybeSuggest(b: BatteryLike): void {
  if (suggested || b.charging || b.level >= 0.2) return;
  const { ecoMode, pushToast } = useUiStore.getState();
  if (ecoMode) return;
  suggested = true;
  pushToast({
    message: `Battery at ${Math.round(b.level * 100)}% — Eco mode (Settings → Appearance) extends it.`,
    tone: "info",
  });
}

export function debugSuggest(level = 0.15): void {
  maybeSuggest({ level, charging: false, addEventListener() {} });
}

export function initBatterySuggest(): () => void {
  if (import.meta.env.DEV) {
    const g = window as unknown as { __adios?: Record<string, unknown> };
    if (g.__adios) g.__adios.battery = { suggest: debugSuggest };
  }

  const nav = navigator as NavigatorBattery;
  if (!nav.getBattery) return () => {};
  let battery: BatteryLike | null = null;
  let disposed = false;
  const onChange = () => {
    if (battery) maybeSuggest(battery);
  };
  void nav
    .getBattery()
    .then((b) => {
      if (disposed) return;
      battery = b;
      b.addEventListener("levelchange", onChange);
      b.addEventListener("chargingchange", onChange);
      maybeSuggest(b);
    })
    .catch(() => {});
  return () => {
    disposed = true;
  };
}
