import { emit } from "@tauri-apps/api/event";
import { inTauri, trackMediaUrl } from "@/lib/api";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { CrossfadeController, makeSwapAction, nextTrackUrl } from "./crossfade";
import { createEngineElement } from "./element";
import {
  attachElement,
  getGraph,
  peekGraph,
  setElementGain,
  setMasterVolume,
} from "./graph";
import { playWithRetry } from "./playRetry";
import { recoverMediaError } from "./reresolve";

/** Two <audio> A/B elements (crossOrigin="anonymous") on the shared graph.
    Store is the source of truth. Gapless: preload next on the inactive
    element, swap on ended (crossfadeMs=0, D3); crossfadeMs>0 runs an
    equal-power fade via CrossfadeController. */

class PlayerEngine {
  private els: [HTMLAudioElement, HTMLAudioElement] | null = null;
  private active = 0;
  private started = false;
  /** True while element mutation must not echo into store play/pause. */
  private suppress = false;
  /** Monotonic counter so rapid track changes cannot race. */
  private loadGen = 0;
  private cross: CrossfadeController | null = null;

  start(): void {
    if (this.started) return;
    this.started = true;
    this.ensureEls();
    this.cross = new CrossfadeController(
      () => this.els,
      () => this.active,
      makeSwapAction({
        getElements: () => this.els,
        getActiveIndex: () => this.active,
        setActiveIndex: (index) => (this.active = index),
        withSuppress: (fn) => this.withSuppress(fn),
        clearFading: () => {
          if (this.cross) this.cross.fading = false;
        },
      }),
      (fn) => this.withSuppress(fn),
    );
    usePlayerStore.subscribe((state, prev) => {
      if (state.currentTrackId !== prev.currentTrackId) {
        void this.loadCurrent();
      } else if (state.isPlaying !== prev.isPlaying) {
        this.applyPlaying(state.isPlaying);
      }
      if (state.volume !== prev.volume || state.muted !== prev.muted) {
        setMasterVolume(state.volume, state.muted);
      }
      if (state.seekVersion !== prev.seekVersion) {
        this.cross?.cancel();
        const el = this.currentEl();
        if (el) el.currentTime = state.positionSec;
      }
    });
  }

  /** Debug/readout for the smoke harness: [activeGain, inactiveGain]. */
  gainValues(): [number, number] | null {
    const graph = peekGraph();
    if (!graph || !this.els) return null;
    const a = graph.gains.get(this.els[this.active]);
    const b = graph.gains.get(this.els[1 - this.active]);
    return a && b ? [a.gain.value, b.gain.value] : null;
  }

  private async withSuppress(fn: () => Promise<void>): Promise<void> {
    this.suppress = true;
    try {
      await fn();
    } finally {
      this.suppress = false;
    }
  }

  private ensureEls(): [HTMLAudioElement, HTMLAudioElement] {
    if (this.els) return this.els;
    this.els = [
      createEngineElement(this.elementHooks()),
      createEngineElement(this.elementHooks()),
    ];
    return this.els;
  }

  private elementHooks() {
    return {
      currentEl: () => this.currentEl(),
      isSuppressed: () => this.suppress,
      isFading: () => this.cross?.fading ?? false,
      maybeBeginFade: () => void this.cross?.maybeBegin(),
      onError: (el: HTMLAudioElement) => {
        console.error("[engine] media error:", el.error?.message ?? "unknown");
        if (inTauri) void emit("smoke", { msg: `media error: ${el.error?.message}` });
        if (this.suppress) return;
        void recoverMediaError(el, {
          isCurrent: () => el === this.currentEl(),
          resume: () => this.playEl(el),
        }).then((ok) => { if (!ok) usePlayerStore.getState().setPlaying(false); });
      },
    };
  }

  private currentEl(): HTMLAudioElement | null {
    return this.els?.[this.active] ?? null;
  }

  private async loadCurrent(): Promise<void> {
    const gen = ++this.loadGen;
    const els = this.ensureEls();
    const store = usePlayerStore.getState();
    const track = useLibraryStore
      .getState()
      .tracks.find((t) => t.id === store.currentTrackId);
    const url = track ? await trackMediaUrl(track) : null;
    if (gen !== this.loadGen) return;
    if (!url) {
      els.forEach((el) => {
        el.pause();
        el.removeAttribute("src");
        el.load();
      });
      return;
    }

    this.suppress = true;
    try {
      let el = els[this.active];
      const other = els[1 - this.active];
      if (other.src === url) {
        // Preloaded element already buffers this track — swap onto it.
        this.active = 1 - this.active;
        el = els[this.active];
        els[1 - this.active].pause();
      }
      if (el.src !== url) el.src = url;
      const inactive = els[1 - this.active];
      if (!inactive.paused) inactive.pause();
      if (usePlayerStore.getState().isPlaying) await this.playEl(el);
      void this.preloadNext();
    } finally {
      this.suppress = false;
    }
  }

  private async playEl(el: HTMLAudioElement): Promise<void> {
    try {
      const graph = getGraph();
      attachElement(el);
      setElementGain(el, 1);
      const other = this.els?.[1 - this.active];
      if (other && !this.cross?.fading) setElementGain(other, 0);
      if (graph.ctx.state === "suspended") await graph.ctx.resume().catch(() => {});
      const { volume, muted } = usePlayerStore.getState();
      setMasterVolume(volume, muted);
      await playWithRetry(el);
    } catch (error) {
      console.error("[engine] play() rejected:", error);
      if (inTauri) void emit("smoke", { msg: `play() rejected: ${String(error)}` });
      usePlayerStore.getState().setPlaying(false);
    }
  }

  private applyPlaying(playing: boolean): void {
    const el = this.currentEl();
    if (!el) return;
    if (playing) void this.playEl(el);
    else {
      this.cross?.cancel();
      if (!el.paused) el.pause();
    }
  }

  /** Buffer the upcoming track on the inactive element (near-gapless swap). */
  private preloadNext(): void {
    const currentId = usePlayerStore.getState().currentTrackId;
    if (!this.els) return;
    void nextTrackUrl().then((url) => {
      if (!url || !this.els) return;
      if (usePlayerStore.getState().currentTrackId !== currentId) return;
      const other = this.els[1 - this.active];
      if (other.src !== url) other.src = url;
    });
  }
}

export const engine = new PlayerEngine();

/** Wire the engine to the store — call once from App. */
export function initPlayerEngine(): void {
  engine.start();
}
