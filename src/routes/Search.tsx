import { useEffect } from "react";
import { useSearchStore } from "@/store/searchStore";

/** The `/search` route exists so the sidebar link works — the actual UI is
    the global Ctrl+K overlay (ARCHITECTURE: "/search also an overlay").
    P9.9: THIS page owns leaving the route. The old design (the store's
    setOpen redirecting the hash + this effect's cleanup calling
    setOpen(false)) fought React StrictMode — setup → cleanup(setOpen(false)
    + hash "#/") → setup → router unmount — so the overlay died instantly
    after the sidebar link opened it and the link looked dead in dev.
    Cleanup now only unsubscribes; closing the overlay while parked here
    navigates back via the store subscription. */
export default function SearchPage() {
  const setOpen = useSearchStore((s) => s.setOpen);
  useEffect(() => {
    setOpen(true);
    const unsub = useSearchStore.subscribe((state, prev) => {
      if (prev.open && !state.open) window.location.hash = "#/";
    });
    return unsub;
  }, [setOpen]);
  return (
    <section className="page">
      <p className="text-ink-3 text-14">Search is open — press Ctrl+K anytime.</p>
    </section>
  );
}
