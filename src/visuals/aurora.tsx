/** P6.2 Aurora backdrop (DESIGN L0): three palette blobs + film grain.
    The drift is pure CSS; this component only owns the on/off gate
    (playing + visible → drift animates, otherwise paused = static frame)
    — band-driven scale/brightness live in aurora.css via `--band-*`. */

import { useEffect, useState } from "react";
import { usePlayerStore } from "@/store/playerStore";
import "./aurora.css";

export function Aurora() {
  const playing = usePlayerStore((s) => s.isPlaying);
  const [visible, setVisible] = useState(
    document.visibilityState === "visible",
  );

  useEffect(() => {
    const onVisibility = () =>
      setVisible(document.visibilityState === "visible");
    document.addEventListener("visibilitychange", onVisibility);
    return () => document.removeEventListener("visibilitychange", onVisibility);
  }, []);

  const on = playing && visible;
  return (
    <div className={`aurora${on ? " aurora--on" : ""}`} aria-hidden="true">
      <span className="aurora__blob aurora__blob--1">
        <span className="aurora__drift" />
      </span>
      <span className="aurora__blob aurora__blob--2">
        <span className="aurora__drift" />
      </span>
      <span className="aurora__blob aurora__blob--3">
        <span className="aurora__drift" />
      </span>
      <span className="aurora__grain" />
    </div>
  );
}
