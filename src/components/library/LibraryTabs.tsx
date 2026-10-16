/** P10: segmented control for the Library's two views. */

const TABS = [
  { id: "downloads", label: "Downloads" },
  { id: "history", label: "History" },
] as const;

export type LibraryTab = (typeof TABS)[number]["id"];

export function LibraryTabs({
  value,
  onChange,
}: {
  value: LibraryTab;
  onChange: (tab: LibraryTab) => void;
}) {
  return (
    <div
      role="tablist"
      aria-label="Library views"
      className="flex gap-1 rounded-ctrl border border-stroke p-1"
    >
      {TABS.map(({ id, label }) => (
        <button
          key={id}
          type="button"
          role="tab"
          aria-selected={value === id}
          onClick={() => onChange(id)}
          className={`rounded-ctrl px-4 py-1.5 text-14 transition ${
            value === id
              ? "bg-accent font-medium text-[color:var(--on-accent)]"
              : "text-ink-2 hover:bg-glass"
          }`}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
