/** Shared Web Audio graph for the two PlayerEngine <audio> elements.

    Element → (source → gain) per element → analyser → master → destination.
    The analyser sits BEFORE master gain so bands keep reacting when the
    volume is lowered (Invariant: crossOrigin="anonymous" elements feed it). */

export interface AudioGraph {
  ctx: AudioContext;
  analyser: AnalyserNode;
  master: GainNode;
  gains: Map<HTMLAudioElement, GainNode>;
}

let graph: AudioGraph | null = null;

/** Create-on-first-use singleton (created lazily so autoplay policy allows it). */
export function getGraph(): AudioGraph {
  if (!graph) {
    const ctx = new AudioContext();
    const analyser = ctx.createAnalyser();
    analyser.fftSize = 2048;
    analyser.smoothingTimeConstant = 0.8;
    const master = ctx.createGain();
    analyser.connect(master);
    master.connect(ctx.destination);
    graph = { ctx, analyser, master, gains: new Map() };
  }
  return graph;
}

/** The graph if it already exists (never creates one). */
export function peekGraph(): AudioGraph | null {
  return graph;
}

/** Wire an element into the graph exactly once; returns its crossfade gain. */
export function attachElement(el: HTMLAudioElement): GainNode {
  const g = getGraph();
  const existing = g.gains.get(el);
  if (existing) return existing;
  const source = g.ctx.createMediaElementSource(el);
  const gain = g.ctx.createGain();
  source.connect(gain);
  gain.connect(g.analyser);
  g.gains.set(el, gain);
  return gain;
}

/** Null until the first element is attached (first play). */
export function getAnalyser(): AnalyserNode | null {
  return graph?.analyser ?? null;
}
