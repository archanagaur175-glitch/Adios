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
