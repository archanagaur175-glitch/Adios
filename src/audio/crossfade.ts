/** Equal-power crossfade between the engine's two elements (P3.4 / D3).
    The controller resolves the next track, starts it on the inactive
    element, ramps gains with cosine/sine curves, then calls `swap` once
    the fade completes. `crossfadeMs = 0` never enters this path. */
import { buildMediaUrl } from "@/lib/api";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { peekGraph, attachElement } from "./graph";

const CURVE_POINTS = 33;

function fadeOutCurve(): Float32Array<ArrayBuffer> {
  const curve = new Float32Array(CURVE_POINTS);
  for (let i = 0; i < CURVE_POINTS; i++) {
    curve[i] = Math.cos((Math.PI / 2) * (i / (CURVE_POINTS - 1)));
  }
  return curve;
}

function fadeInCurve(): Float32Array<ArrayBuffer> {
  const curve = new Float32Array(CURVE_POINTS);
  for (let i = 0; i < CURVE_POINTS; i++) {
    curve[i] = Math.sin((Math.PI / 2) * (i / (CURVE_POINTS - 1)));
  }
  return curve;
}

/** Ramp `from` 1→0 and `to` 0→1 over `ms` (equal-power curves). */
export function startEqualPowerFade(
  ctx: AudioContext,
  from: GainNode,
  to: GainNode,
  ms: number,
): void {
  const now = ctx.currentTime;
  from.gain.cancelScheduledValues(now);
  to.gain.cancelScheduledValues(now);
  from.gain.setValueCurveAtTime(fadeOutCurve(), now, ms / 1000);
  to.gain.setValueCurveAtTime(fadeInCurve(), now, ms / 1000);
}

/** Hard-stop any scheduled automation and settle gains (from=0, to=1). */
export function settleGains(ctx: AudioContext, from: GainNode, to: GainNode): void {
  const now = ctx.currentTime;
  from.gain.cancelScheduledValues(now);
  to.gain.cancelScheduledValues(now);
  from.gain.setValueAtTime(0, now);
  to.gain.setValueAtTime(1, now);
}

/** Resolve the URL of the upcoming track (shared by preload + fade). */
export async function nextTrackUrl(): Promise<string | null> {
  const store = usePlayerStore.getState();
  const nextId = store.peekNextId();
  if (nextId == null) return null;
  const track = useLibraryStore
    .getState()
    .tracks.find((t) => t.id === nextId);
  if (!track?.path) return null;
  return buildMediaUrl(track.path);
}

/** Advance the queue after a completed fade; stop at queue end. */
export function advanceQueueAfterFade(): void {
  const store = usePlayerStore.getState();
  const before = store.currentTrackId;
  store.next();
  if (usePlayerStore.getState().currentTrackId === before) {
    usePlayerStore.getState().setPlaying(false);
  }
}

export type SwapDeps = {
  getElements: () => [HTMLAudioElement, HTMLAudioElement] | null;
  getActiveIndex: () => number;
  setActiveIndex: (index: number) => void;
  withSuppress: (fn: () => Promise<void>) => Promise<void>;
  clearFading: () => void;
};

/** The post-fade A/B swap: pause old, settle gains, flip active, advance. */
export function makeSwapAction(deps: SwapDeps): () => void {
  return () => {
    void deps.withSuppress(async () => {
      const els = deps.getElements();
      if (!els) return;
      const graph = peekGraph();
      const from = els[deps.getActiveIndex()];
      const to = els[1 - deps.getActiveIndex()];
      from.pause();
      if (graph) settleGains(graph.ctx, graph.gains.get(from)!, graph.gains.get(to)!);
      deps.setActiveIndex(1 - deps.getActiveIndex());
      deps.clearFading();
    });
    advanceQueueAfterFade();
  };
}

/** Drives one crossfade; engine supplies element access + the final swap. */
export class CrossfadeController {
  fading = false;

  constructor(
    private readonly getElements: () => [HTMLAudioElement, HTMLAudioElement] | null,
    private readonly getActiveIndex: () => number,
    private readonly swap: () => void,
    private readonly withSuppress: (fn: () => Promise<void>) => Promise<void>,
  ) {}

  /** Begin a fade when within `crossfadeMs` of the active track's end. */
  async maybeBegin(): Promise<void> {
    if (this.fading) return;
    const els = this.getElements();
    if (!els) return;
    const store = usePlayerStore.getState();
    if (store.crossfadeMs <= 0) return;
    const from = els[this.getActiveIndex()];
    if (!Number.isFinite(from.duration) || from.duration <= 0) return;
    if (from.duration - from.currentTime > store.crossfadeMs / 1000) return;

    const trackIdAtStart = store.currentTrackId;
    const url = await nextTrackUrl();
    if (!url || this.fading) return;
    if (usePlayerStore.getState().currentTrackId !== trackIdAtStart) return;

    const graph = peekGraph();
    if (!graph) return;
    const to = els[1 - this.getActiveIndex()];
    if (to.src !== url) to.src = url;
    try {
      to.currentTime = 0;
    } catch {
      // seeking before metadata: ignore, play() will start from 0
    }

    this.fading = true;
    await this.withSuppress(async () => {
      const toGain = attachElement(to);
      toGain.gain.value = 0;
      const fromGain = graph.gains.get(from);
      if (!fromGain) {
        this.fading = false;
        return;
      }
      try {
        await to.play();
      } catch (error) {
        this.fading = false;
        console.error("[crossfade] next play() rejected:", error);
        return;
      }
      startEqualPowerFade(graph.ctx, fromGain, toGain, store.crossfadeMs);
      window.setTimeout(() => {
        if (this.fading) this.swap();
      }, store.crossfadeMs + 40);
    });
  }

  /** Abort a running fade: pause the incoming element, settle gains. */
  cancel(): void {
    if (!this.fading) return;
    const els = this.getElements();
    const graph = peekGraph();
    this.fading = false;
    if (!els || !graph) return;
    const from = els[this.getActiveIndex()];
    const to = els[1 - this.getActiveIndex()];
    to.pause();
    const fromGain = graph.gains.get(from);
    const toGain = graph.gains.get(to);
    if (fromGain && toGain) settleGains(graph.ctx, fromGain, toGain);
  }
}
