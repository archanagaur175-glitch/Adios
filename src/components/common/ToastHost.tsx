import { useEffect, useRef } from "react";
import { useUiStore } from "@/store/uiStore";

/** Transient toast stack (uiStore) — auto-dismisses after 6 s.
    P4.4 uses it for the stream re-resolve give-up error chip. */
export function ToastHost() {
  const toasts = useUiStore((s) => s.toasts);
  const scheduled = useRef(new Set<number>());

  useEffect(() => {
    for (const toast of toasts) {
      if (scheduled.current.has(toast.id)) continue;
      scheduled.current.add(toast.id);
      setTimeout(() => useUiStore.getState().dismissToast(toast.id), 6000);
    }
  }, [toasts]);

  if (toasts.length === 0) return null;
  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-50 flex flex-col items-end gap-2">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          role="status"
          className={
            toast.tone === "danger"
              ? "rounded-card bg-danger px-3 py-2 text-14 text-bg-0 shadow-lg"
              : "rounded-card bg-accent px-3 py-2 text-14 text-bg-0 shadow-lg"
          }
        >
          {toast.message}
        </div>
      ))}
    </div>
  );
}
