/** P6.3 host: one full-bleed canvas behind content (World layer, above the
    aurora blobs). Gated on playing + visibility + tier — Eco hides and
    clears; paused keeps the last drawn frame (ARCHITECTURE §8). Also owns
    the `.eco` class on :root (solid panels, static aurora via viz.css). */

import { useEffect, useRef } from "react";
import { usePlayerStore } from "@/store/playerStore";
import { useUiStore } from "@/store/uiStore";
import { tierConfig } from "./tiers";
import { createVisualizer, type VizController } from "./visualizer2d";
import "./viz.css";

export function VisualizerHost() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctrl: VizController = createVisualizer(canvas);

    const sync = () => {
      const { qualityTier, ecoMode } = useUiStore.getState();
      ctrl.setConfig(tierConfig(qualityTier, ecoMode));
      document.documentElement.classList.toggle("eco", ecoMode);
      canvas.hidden = ecoMode;
      if (ecoMode) {
        ctrl.clear();
        return;
      }
      const active =
        usePlayerStore.getState().isPlaying &&
        document.visibilityState === "visible";
      if (active) ctrl.start();
      else ctrl.stop(); // paused: keep the last static frame
    };

    const onVis = () => sync();
    const unsubPlayer = usePlayerStore.subscribe(() => sync());
    const unsubUi = useUiStore.subscribe(() => sync());
    document.addEventListener("visibilitychange", onVis);
    sync(); // initial: reconcile with persisted qualityTier

    if (import.meta.env.DEV) {
      const g = window as unknown as { __adios?: Record<string, unknown> };
      if (g.__adios) {
        g.__adios.viz = {
          get fps() {
            return ctrl.lastFps;
          },
        };
      }
    }

    return () => {
      unsubPlayer();
      unsubUi();
      document.removeEventListener("visibilitychange", onVis);
      ctrl.clear();
    };
  }, []);

  return <canvas ref={canvasRef} className="viz" aria-hidden="true" />;
}
