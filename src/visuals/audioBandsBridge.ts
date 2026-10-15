/** P6.1 audioBands bridge: P3 analyser → `--band-*` CSS custom properties
    on `:root`, one rAF loop that runs ONLY while `isPlaying && visible`
    (Invariant: no background rAF). Every start/stop logs `[bands] …` so the
    gate can prove visibility gating; writes are style-property pokes — no
    React re-renders ever come from this loop. */

import { emit } from "@tauri-apps/api/event";
import { readBands } from "@/audio/bands";
import { getAnalyser } from "@/audio/graph";
import { inTauri } from "@/lib/api";
import { usePlayerStore } from "@/store/playerStore";

let rafId = 0;
let running = false;

/** Devtools console + (inside Tauri) a `bands` event the Rust side prints
    as `[bands] …` — webview console output never reaches the dev log. */
function log(msg: string): void {
  console.log(`[bands] ${msg}`);
  if (inTauri) void emit("bands", { msg });
}

function setVar(name: string, value: string): void {
  document.documentElement.style.setProperty(name, value);
}

function shouldRun(): boolean {
  return usePlayerStore.getState().isPlaying && document.visibilityState === "visible";
}

function frame(): void {
  if (!running) return;
  const analyser = getAnalyser();
  if (analyser) {
    const b = readBands(analyser);
    setVar("--band-bass", b.bass.toFixed(3));
    setVar("--band-mid", b.mid.toFixed(3));
    setVar("--band-high", b.high.toFixed(3));
    setVar("--band-rms", b.rms.toFixed(3));
  }
  rafId = requestAnimationFrame(frame);
}

function start(): void {
  if (running) return;
  running = true;
  rafId = requestAnimationFrame(frame);
  log("start (playing + visible)");
}

/** Stop the loop and rest the vars at baseline (consumers read 0 while idle). */
function stop(reason: string): void {
  if (!running) return;
  running = false;
  cancelAnimationFrame(rafId);
  setVar("--band-bass", "0");
  setVar("--band-mid", "0");
  setVar("--band-high", "0");
  setVar("--band-rms", "0");
  log(`stop (${reason})`);
}

function sync(trigger: string): void {
  if (shouldRun()) start();
  else if (running) stop(trigger);
}

/** Install store/visibility listeners; call once from App (returns cleanup). */
export function initBandsBridge(): () => void {
  const unsub = usePlayerStore.subscribe((state, prev) => {
    if (state.isPlaying !== prev.isPlaying) {
      sync(state.isPlaying ? "playing" : "paused");
    }
  });
  const onVisibility = () =>
    sync(document.visibilityState === "visible" ? "restored" : "hidden");
  document.addEventListener("visibilitychange", onVisibility);
  sync("init");

  return () => {
    unsub();
    document.removeEventListener("visibilitychange", onVisibility);
    stop("dispose");
  };
}
