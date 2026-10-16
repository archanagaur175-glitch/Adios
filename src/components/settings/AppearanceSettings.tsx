import { useSettingsStore } from "@/store/settingsStore";
import { useUiStore } from "@/store/uiStore";
import type { QualityTier } from "@/lib/types";

const selectCls =
  "rounded-ctrl border border-stroke bg-glass px-3 py-1.5 text-14 text-ink-1 focus:border-accent";

const TIERS: { value: QualityTier; label: string }[] = [
  { value: "low", label: "Low — 48 bars, 30 fps" },
  { value: "med", label: "Med — 96 bars, 60 fps, soft glow" },
  { value: "high", label: "High — 128 bars, 60 fps, full glow" },
];

/** Appearance (DESIGN §7): pinned quality tier + Eco toggle.
    Battery saver auto-suggests Eco (P6.3); reduced motion follows the OS. */
export function AppearanceSettings() {
  const qualityTier = useUiStore((s) => s.qualityTier);
  const ecoMode = useUiStore((s) => s.ecoMode);
  const setQualityTier = useUiStore((s) => s.setQualityTier);
  const setEcoMode = useUiStore((s) => s.setEcoMode);
  const setSettings = useSettingsStore((s) => s.set);

  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-18 font-display">Appearance</h2>

      <div className="flex items-center gap-2">
        <label htmlFor="viz-tier" className="w-36 text-13 text-ink-2">
          Visualizer tier
        </label>
        <select
          id="viz-tier"
          value={qualityTier}
          onChange={(e) => {
            const tier = e.target.value as QualityTier;
            setQualityTier(tier);
            setSettings({ defaultTier: tier });
          }}
          className={`${selectCls} min-w-0 flex-1`}
        >
          {TIERS.map((t) => (
            <option key={t.value} value={t.value}>
              {t.label}
            </option>
          ))}
        </select>
      </div>

      <label className="flex items-center gap-2 text-13 text-ink-2">
        <input
          type="checkbox"
          checked={ecoMode}
          onChange={(e) => setEcoMode(e.target.checked)}
          className="accent-accent"
        />
        Eco mode — visualizer off, static aurora, solid panels (suggested
        automatically on low battery)
      </label>

      <p className="text-12 text-ink-3">
        Reduced motion follows your operating-system setting.
      </p>
    </section>
  );
}
