/** Minimal inline transport icons (no icon dependency). */

type Props = { className?: string };

const base = {
  width: 20,
  height: 20,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.8,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
};

export function IconPlay({ className }: Props) {
  return (
    <svg {...base} className={className} fill="currentColor" stroke="none">
      <path d="M8 5.5v13l11-6.5z" />
    </svg>
  );
}

/** P15.3: playlist-add (list + plus) for the player bar. */
export function IconAddToList({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <line x1="3" y1="6" x2="13" y2="6" />
      <line x1="3" y1="12" x2="10" y2="12" />
      <line x1="3" y1="18" x2="8" y2="18" />
      <line x1="17" y1="13" x2="17" y2="21" />
      <line x1="13" y1="17" x2="21" y2="17" />
    </svg>
  );
}

export function IconPause({ className }: Props) {
  return (
    <svg {...base} className={className} fill="currentColor" stroke="none">
      <rect x="6.5" y="5.5" width="3.6" height="13" rx="1" />
      <rect x="13.9" y="5.5" width="3.6" height="13" rx="1" />
    </svg>
  );
}

export function IconPrev({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <path d="M18 6v12L9 12z" fill="currentColor" stroke="none" />
      <line x1="6" y1="6" x2="6" y2="18" />
    </svg>
  );
}

export function IconNext({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <path d="M6 6v12l9-6z" fill="currentColor" stroke="none" />
      <line x1="18" y1="6" x2="18" y2="18" />
    </svg>
  );
}

export function IconVolume({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <path d="M4 9.5v5h3.5L12 18V6L7.5 9.5z" fill="currentColor" stroke="none" />
      <path d="M15.5 9a4.5 4.5 0 0 1 0 6" />
      <path d="M18 6.5a8 8 0 0 1 0 11" />
    </svg>
  );
}

export function IconMute({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <path d="M4 9.5v5h3.5L12 18V6L7.5 9.5z" />
      <line x1="15.5" y1="9" x2="20.5" y2="14.5" />
      <line x1="20.5" y1="9" x2="15.5" y2="14.5" />
    </svg>
  );
}

export function IconLyrics({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <line x1="4" y1="6" x2="14" y2="6" />
      <line x1="4" y1="11" x2="20" y2="11" />
      <line x1="4" y1="16" x2="9" y2="16" />
      <circle cx="15.5" cy="17" r="2.4" />
      <path d="M17.9 17V8.8l3 .9" />
    </svg>
  );
}

export function IconShuffle({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <path d="M3 6h3.5l3 4" />
      <path d="M14.5 14H18l3 4" />
      <path d="M18 6h3v3" />
      <path d="M3 18h3.5l8-12H18" />
      <path d="M18 18h3v-3" />
      <path d="M18 6l3 3-3 3" />
      <path d="M18 15l3 3-3 3" />
    </svg>
  );
}

export function IconRepeat({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <path d="M17 3l3 3-3 3" />
      <path d="M4 12V10a4 4 0 0 1 4-4h12" />
      <path d="M7 21l-3-3 3-3" />
      <path d="M20 12v2a4 4 0 0 1-4 4H4" />
    </svg>
  );
}

export function IconRepeatOne({ className }: Props) {
  return (
    <svg {...base} className={className}>
      <path d="M17 3l3 3-3 3" />
      <path d="M4 12V10a4 4 0 0 1 4-4h12" />
      <path d="M7 21l-3-3 3-3" />
      <path d="M20 12v2a4 4 0 0 1-4 4H4" />
      <path d="M11 10.5l1.5-1v4" />
    </svg>
  );
}
