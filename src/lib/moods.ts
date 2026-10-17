/** P16 (B): curated mood seeds for Home's Moods grid → `#/moods/:id`.
    Pure data + lookup (unit-testable); the query runs through the Rust
    iTunes cache when a mood page opens. Card color comes from `gradient`
    only — color-only styling, never a new backdrop-filter (backdrop budget
    stays 4/4). */

export interface Mood {
  id: string;
  name: string;
  /** iTunes search seed (relevance order; cached Rust-side forever). */
  query: string;
  /** CSS background for the Home card (color-only, no blur). */
  gradient: string;
}

export const MOODS: Mood[] = [
  { id: "chill", name: "Chill", query: "chill", gradient: "linear-gradient(135deg, #1e3a5f, #38bdf8)" },
  { id: "focus", name: "Focus", query: "focus instrumental", gradient: "linear-gradient(135deg, #312e81, #818cf8)" },
  { id: "workout", name: "Workout", query: "workout", gradient: "linear-gradient(135deg, #7f1d1d, #f97316)" },
  { id: "party", name: "Party", query: "party hits", gradient: "linear-gradient(135deg, #701a75, #ec4899)" },
  { id: "sad", name: "Sad", query: "sad songs", gradient: "linear-gradient(135deg, #0f172a, #64748b)" },
  { id: "sleep", name: "Sleep", query: "sleep", gradient: "linear-gradient(135deg, #1e1b4b, #4c1d95)" },
  { id: "drive", name: "Drive", query: "road trip", gradient: "linear-gradient(135deg, #14532d, #facc15)" },
  { id: "coffee", name: "Coffee", query: "acoustic coffee", gradient: "linear-gradient(135deg, #431407, #d97706)" },
  { id: "indie", name: "Indie", query: "indie", gradient: "linear-gradient(135deg, #134e4a, #2dd4bf)" },
  { id: "oldies", name: "Oldies", query: "oldies", gradient: "linear-gradient(135deg, #422006, #a16207)" },
];

export function moodById(id: string | undefined): Mood | null {
  if (!id) return null;
  return MOODS.find((m) => m.id === id) ?? null;
}
