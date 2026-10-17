import { convertFileSrc } from "@tauri-apps/api/core";
import { inTauri } from "@/lib/api";

/** P12: shared cover thumbnail for every list surface. Resolves the best
    available image — local cover file (asset protocol, scoped to the
    covers dir in tauri.conf.json) → remote artwork URL (iTunes) → ♪
    placeholder. Rows render ≤ 40 px; hero surfaces keep their own imgs. */

export interface CoverSource {
  coverPath?: string | null;
  artworkUrl?: string | null;
}

export function coverSrcFor(track: CoverSource): string | null {
  if (track.coverPath && inTauri) return convertFileSrc(track.coverPath);
  if (track.artworkUrl) return track.artworkUrl;
  return null;
}

export function CoverArt({
  track,
  size = 36,
  className = "",
}: {
  track: CoverSource;
  size?: number;
  className?: string;
}) {
  const src = coverSrcFor(track);
  if (!src) {
    return (
      <span
        aria-hidden="true"
        className={`flex shrink-0 items-center justify-center rounded-card bg-glass text-ink-3 ${className}`}
        style={{ width: size, height: size, fontSize: Math.round(size * 0.42) }}
      >
        ♪
      </span>
    );
  }
  return (
    <img
      src={src}
      alt=""
      draggable={false}
      loading="lazy"
      className={`shrink-0 rounded-card object-cover ${className}`}
      style={{ width: size, height: size }}
    />
  );
}
