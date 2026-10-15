import { emit } from "@tauri-apps/api/event";
import { buildMediaUrl, inTauri } from "@/lib/api";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { attachElement, getGraph, peekGraph } from "./graph";

/** Two <audio> A/B elements (both crossOrigin="anonymous") wired into the
    shared graph. Store is the source of truth; engine applies changes and
    reports events back. Gapless path: preload next on the inactive element,
    swap on ended (crossfadeMs=0 = preload-and-switch, D3). */

class PlayerEngine {
  private els: [HTMLAudioElement, HTMLAudioElement] | null = null;
  private active = 0;
  private started = false;
  /** True while loadCurrent mutates elements (suppresses play/pause echoes). */
  private suppress = false;
  /** Monotonic counter so rapid track changes cannot race. */
  private loadGen = 0;

  start(): void {
    if (this.started) return;
    this.started = true;
    this.ensureEls();
    usePlayerStore.subscribe((state, prev) => {
      if (state.currentTrackId !== prev.currentTrackId) {
        void this.loadCurrent();
      } else if (state.isPlaying !== prev.isPlaying) {
        this.applyPlaying(state.isPlaying);
      }
      if (state.volume !== prev.volume || state.muted !== prev.muted) {
        this.applyVolume();
      }
      if (state.seekVersion !== prev.seekVersion) {
        const el = this.currentEl();
        if (el) el.currentTime = state.positionSec;
      }
    });
  }

  private ensureEls(): [HTMLAudioElement, HTMLAudioElement] {
    if (this.els) return this.els;
    const make = () => {
      const el = new Audio();
      el.crossOrigin = "anonymous";
      el.preload = "auto";
      el.addEventListener("ended", () => this.onEnded());
      el.addEventListener("timeupdate", () => this.onTimeUpdate(el));
      el.addEventListener("loadedmetadata", () => this.onMeta(el));
      el.addEventListener("play", () => {
        if (!this.suppress && el === this.currentEl()) {
          usePlayerStore.getState().setPlaying(true);
        }
      });
      el.addEventListener("pause", () => {
        if (!this.suppress && el === this.currentEl()) {
          usePlayerStore.getState().setPlaying(false);
        }
      });
      el.addEventListener("error", () => {
        if (!this.suppress && el === this.currentEl()) {
          console.error("[engine] media error:", el.error?.message ?? "unknown");
          usePlayerStore.getState().setPlaying(false);
          if (inTauri) void emit("smoke", { msg: `media error: ${el.error?.message}` });
        }
      });
      return el;
    };
    this.els = [make(), make()];
    return this.els;
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
    const url = track?.path
      ? await buildMediaUrl(track.path)
      : (track?.url ?? null);
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
        this.active = 1 - this.active;
        el = els[this.active];
        els[1 - this.active].pause();
      }
      if (el.src !== url) el.src = url;
      const inactive = els[1 - this.active];
      if (!inactive.paused) inactive.pause();
      if (usePlayerStore.getState().isPlaying) await this.playEl(el);
      this.preloadNext();
    } finally {
      this.suppress = false;
    }
  }

  private async playEl(el: HTMLAudioElement): Promise<void> {
    try {
      attachElement(el);
      const ctx = getGraph().ctx;
      if (ctx.state === "suspended") await ctx.resume().catch(() => {});
      this.applyVolume();
      await el.play();
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
    else if (!el.paused) el.pause();
  }

  private applyVolume(): void {
    const graph = this.els ? peekGraph() : null;
    if (!graph) return;
    const { volume, muted } = usePlayerStore.getState();
    graph.master.gain.value = muted ? 0 : volume;
  }

  private onTimeUpdate(el: HTMLAudioElement): void {
    if (el !== this.currentEl() || this.suppress) return;
    const store = usePlayerStore.getState();
    if (Math.abs(el.currentTime - store.positionSec) >= 0.25) {
      store.setPosition(el.currentTime);
    }
  }

  private onMeta(el: HTMLAudioElement): void {
    if (el !== this.currentEl()) return;
    const duration = Number.isFinite(el.duration) ? el.duration : 0;
    usePlayerStore.getState().setDuration(duration);
  }

  private onEnded(): void {
    const store = usePlayerStore.getState();
    if (store.repeatMode === "track") {
      const el = this.currentEl();
      if (el) {
        el.currentTime = 0;
        void el.play().catch(() => {});
      }
      return;
    }
    const before = store.currentTrackId;
    store.next();
    const after = usePlayerStore.getState();
    if (after.currentTrackId === before) after.setPlaying(false);
  }

  /** Buffer the upcoming track on the inactive element (near-gapless swap). */
  private preloadNext(): void {
    const store = usePlayerStore.getState();
    const nextId = store.peekNextId();
    const currentId = store.currentTrackId;
    if (nextId == null || !this.els) return;
    const track = useLibraryStore.getState().tracks.find((t) => t.id === nextId);
    if (!track?.path) return;
    void buildMediaUrl(track.path).then((url) => {
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
