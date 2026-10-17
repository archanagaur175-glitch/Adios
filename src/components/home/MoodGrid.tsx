import { Link } from "react-router";
import { MOODS } from "@/lib/moods";

/** P16 section B: the Moods grid — ten color-only gradient cards linking to
    `#/moods/:id` (the search itself runs when the mood opens, not here).
    No new backdrop-filter anywhere: gradients are plain CSS backgrounds. */

export function MoodGrid() {
  return (
    <section>
      <header className="mb-3 flex items-end justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-18 font-display">Moods</h2>
          <p className="text-12 text-ink-3">Pick a vibe — it loads when you open it</p>
        </div>
      </header>
      <ul className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
        {MOODS.map((m) => (
          <li key={m.id}>
            <Link
              to={`/moods/${m.id}`}
              className="flex h-20 items-end rounded-card p-3 text-14 font-medium text-white/90 shadow-sm transition hover:opacity-85"
              style={{ background: m.gradient }}
            >
              {m.name}
            </Link>
          </li>
        ))}
      </ul>
    </section>
  );
}
