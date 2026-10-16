/** P6.3 2D visualizer (DESIGN L1): radial ring + bars + waveform, additive
    `lighter`; per-tier fps-cap rAF gated on playing + visible (paused keeps
    last frame). Colors refresh ~500 ms; `[viz] fps=…` every 2 s via event. */

import { emit } from "@tauri-apps/api/event";
import { getAnalyser } from "@/audio/graph";
import { inTauri } from "@/lib/api";
import type { TierConfig } from "./tiers";

export interface VizController {
  setConfig(cfg: TierConfig): void;
  start(): void;
  stop(): void;
  /** Clear the canvas (Eco / dispose); plain stop keeps the last frame. */
  clear(): void;
  /** Play/pause accent pulse (DESIGN §8): briefly boost the glow pass. */
  pulse(): void;
  readonly lastFps: number;
  readonly boostRemaining: number; // ms left on the current pulse (dev hook)
}

const REPORT_MS = 2000;

export function createVisualizer(canvas: HTMLCanvasElement): VizController {
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("canvas 2d context unavailable");

  let cfg: TierConfig = { bars: 96, fpsCap: 60, glow: "soft" };
  let raf = 0;
  let running = false;
  let lastDraw = 0;
  let frames = 0;
  let windowStart = 0;
  let lastFps = 0;
  let boostUntil = 0;
  let colorCheck = 0;
  let accent = "#7c5cff";
  let accent2 = "#5546c8";
  let freq: Uint8Array<ArrayBuffer> | null = null;
  let time: Uint8Array<ArrayBuffer> | null = null;

  function refreshColors(now: number): void {
    if (now - colorCheck < 500) return;
    colorCheck = now;
    const cs = getComputedStyle(document.documentElement);
    accent = cs.getPropertyValue("--palette-accent").trim() || accent;
    accent2 = cs.getPropertyValue("--palette-accent2").trim() || accent2;
  }

  function resize(): void {
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5); // budget: DPR ≤ 1.5
    const pw = Math.max(1, Math.round(canvas.clientWidth * dpr));
    const ph = Math.max(1, Math.round(canvas.clientHeight * dpr));
    if (canvas.width !== pw || canvas.height !== ph) {
      canvas.width = pw;
      canvas.height = ph;
    }
  }

  function draw(now: number): void {
    const analyser = getAnalyser();
    if (!ctx || !analyser || cfg.bars <= 0) return;
    if (!freq || freq.length !== analyser.frequencyBinCount) {
      freq = new Uint8Array(analyser.frequencyBinCount);
    }
    if (!time || time.length !== analyser.fftSize) {
      time = new Uint8Array(analyser.fftSize);
    }
    analyser.getByteFrequencyData(freq);
    analyser.getByteTimeDomainData(time);
    refreshColors(now);

    const w = canvas.width;
    const h = canvas.height;
    const cx = w / 2;
    const cy = h / 2;
    const unit = Math.min(w, h);
    const rBase = unit * 0.16;

    let sum = 0;
    for (let i = 0; i < time.length; i++) {
      const v = time[i] / 128 - 1;
      sum += v * v;
    }
    const rms = Math.sqrt(sum / time.length);

    ctx.clearRect(0, 0, w, h);
    ctx.globalCompositeOperation = "lighter";

    // Bars radiating from the ring (log-mapped spectrum).
    const maxBar = unit * 0.24;
    const step = (Math.PI * 2) / cfg.bars;
    ctx.strokeStyle = accent;
    ctx.lineWidth = Math.max(2, ((Math.PI * 2 * rBase) / cfg.bars) * 0.55);
    ctx.lineCap = "round";
    ctx.beginPath();
    for (let i = 0; i < cfg.bars; i++) {
      const bin = Math.min(
        freq.length - 1,
        Math.floor(Math.pow(i / cfg.bars, 1.7) * freq.length * 0.6),
      );
      const v = freq[bin] / 255;
      const ang = i * step - Math.PI / 2;
      const r0 = rBase * (1 + rms * 0.25);
      const r1 = r0 + v * maxBar * (1 + rms * 0.5);
      ctx.moveTo(cx + Math.cos(ang) * r0, cy + Math.sin(ang) * r0);
      ctx.lineTo(cx + Math.cos(ang) * r1, cy + Math.sin(ang) * r1);
    }
    ctx.stroke();

    // Waveform ring (thin, secondary color).
    ctx.globalAlpha = 0.9;
    ctx.strokeStyle = accent2;
    ctx.lineWidth = 2;
    ctx.beginPath();
    const wR = rBase * 0.72;
    const pts = 128;
    for (let i = 0; i <= pts; i++) {
      const ang = (i / pts) * Math.PI * 2 - Math.PI / 2;
      const v = time[Math.floor((i / pts) * time.length)] / 128 - 1;
      const r = wR + v * unit * 0.03;
      const x = cx + Math.cos(ang) * r;
      const y = cy + Math.sin(ang) * r;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.closePath();
    ctx.stroke();
    ctx.globalAlpha = 1;

    // Glow pass (Med soft halo; High fatter + brighter; pulse boosts both).
    if (cfg.glow !== "off") {
      const boost = now < boostUntil ? 1 - (boostUntil - now) / 500 : 0;
      ctx.save();
      ctx.globalAlpha = (cfg.glow === "full" ? 0.5 : 0.3) * (1 + boost * 1.5);
      ctx.shadowColor = accent;
      ctx.shadowBlur = (cfg.glow === "full" ? 18 : 9) + boost * 10;
      ctx.lineWidth = unit * (cfg.glow === "full" ? 0.012 : 0.008);
      ctx.strokeStyle = accent2;
      ctx.beginPath();
      ctx.arc(cx, cy, rBase * (1 + rms * 0.35), 0, Math.PI * 2);
      ctx.stroke();
      ctx.restore();
    }

    ctx.globalCompositeOperation = "source-over";
  }

  function frame(now: number): void {
    if (!running) return;
    const interval = 1000 / cfg.fpsCap;
    if (now - lastDraw >= interval) {
      lastDraw = now;
      draw(now);
      frames += 1;
      if (now - windowStart >= REPORT_MS) {
        lastFps = Math.round((frames * 1000) / (now - windowStart));
        frames = 0;
        windowStart = now;
        const msg = `fps=${lastFps} bars=${cfg.bars} cap=${cfg.fpsCap} glow=${cfg.glow}`;
        console.log(`[viz] ${msg}`);
        if (inTauri) void emit("viz", { msg });
      }
    }
    raf = requestAnimationFrame(frame);
  }

  return {
    setConfig(next) { cfg = next; },
    start() {
      if (running) return;
      if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
      resize();
      running = true;
      lastDraw = 0;
      frames = 0;
      windowStart = performance.now();
      raf = requestAnimationFrame(frame);
    },
    stop() {
      if (!running) return;
      running = false;
      cancelAnimationFrame(raf);
    },
    clear() {
      this.stop();
      ctx?.clearRect(0, 0, canvas.width, canvas.height);
    },
    pulse() {
      boostUntil = performance.now() + 500;
    },
    get lastFps() {
      return lastFps;
    },
    get boostRemaining() {
      return Math.max(0, boostUntil - performance.now());
    },
  };
}
