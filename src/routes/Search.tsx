import { useEffect } from "react";
import { useSearchStore } from "@/store/searchStore";

/** The `/search` route exists so the sidebar link works — the actual UI is
    the global Ctrl+K overlay (ARCHITECTURE: "/search also an overlay"). */
export default function SearchPage() {
  const setOpen = useSearchStore((s) => s.setOpen);
  useEffect(() => {
    setOpen(true);
    return () => setOpen(false);
  }, [setOpen]);
  return (
    <section className="page">
      <p className="text-ink-3 text-14">Search is open — press Ctrl+K anytime.</p>
    </section>
  );
}
