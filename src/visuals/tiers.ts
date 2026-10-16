/** P6.3 quality-tier table (DESIGN §7 / ARCHITECTURE §8):
    Low 48 bars/30 fps/glow off · Med 96/60/soft · High 128/60/full ·
    Eco = visualizer off, 30 fps cap, static aurora, no backdrop-filter. */

import type { QualityTier } from "@/lib/types";

export interface TierConfig {
  bars: number;
  fpsCap: number;
  glow: "off" | "soft" | "full";
}

const TIERS: Record<QualityTier, TierConfig> = {
  low: { bars: 48, fpsCap: 30, glow: "off" },
  med: { bars: 96, fpsCap: 60, glow: "soft" },
  high: { bars: 128, fpsCap: 60, glow: "full" },
};

export const ECO_CONFIG: TierConfig = { bars: 0, fpsCap: 30, glow: "off" };

export function tierConfig(tier: QualityTier, eco = false): TierConfig {
  return eco ? ECO_CONFIG : TIERS[tier];
}
