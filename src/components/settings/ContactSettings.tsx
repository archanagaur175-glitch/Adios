import { useEffect, useState } from "react";
import { useSettingsStore } from "@/store/settingsStore";

const inputCls =
  "rounded-ctrl border border-stroke bg-glass px-3 py-1.5 text-14 text-ink-1 focus:border-accent";
const btnCls =
  "rounded-ctrl bg-accent px-3 py-1.5 text-13 font-medium text-[color:var(--on-accent)] transition hover:opacity-90";

/** MusicBrainz contact email (D2 / P5.3): feeds the MB User-Agent; without
    it Adios sends a placeholder and shows a one-time warning. */
export function ContactSettings() {
  const contactEmail = useSettingsStore((s) => s.contactEmail);
  const mbWarningShown = useSettingsStore((s) => s.mbWarningShown);
  const setSettings = useSettingsStore((s) => s.set);
  const [draft, setDraft] = useState(contactEmail);
  const [savedNote, setSavedNote] = useState(false);

  // Hydration (and saves from elsewhere) flow back into the draft.
  useEffect(() => setDraft(contactEmail), [contactEmail]);

  const showWarning = !contactEmail && !mbWarningShown;

  function save() {
    setSettings({ contactEmail: draft.trim() });
    setSavedNote(true);
    window.setTimeout(() => setSavedNote(false), 2000);
  }

  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-18 font-display">Metadata</h2>

      {showWarning && (
        <div className="flex items-center justify-between gap-3 rounded-card border border-stroke bg-glass px-3 py-2">
          <p className="text-13 text-ink-2">
            MusicBrainz asks apps to send a contact email in the User-Agent —
            Adios currently sends a placeholder.
          </p>
          <button
            type="button"
            onClick={() => setSettings({ mbWarningShown: true })}
            className="rounded-ctrl border border-stroke px-3 py-1 text-12 text-ink-2 transition hover:bg-glass"
          >
            Dismiss
          </button>
        </div>
      )}

      <div className="flex items-center gap-2">
        <label htmlFor="mb-contact-email" className="w-36 text-13 text-ink-2">
          Contact email
        </label>
        <input
          id="mb-contact-email"
          type="email"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder="you@example.com"
          className={`${inputCls} min-w-0 flex-1`}
        />
        <button type="button" onClick={save} className={btnCls}>
          {savedNote ? "Saved" : "Save"}
        </button>
      </div>

      <p className="text-12 text-ink-3">
        Sent as: Adios/0.1 (personal use; contact: your email) — applies to the
        next MusicBrainz request.
      </p>
    </section>
  );
}
