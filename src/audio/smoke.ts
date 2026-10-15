/** Debug-only verification harness (P3.3/3.4 gates): VITE_ADIOS_SMOKE=1
    plays the first library track, logs AnalyserNode RMS > 0, then
    exercises seek → next → prev → synthetic media-key → pause.
    VITE_ADIOS_XFADE_TEST=1 additionally runs the D3 crossfade test with
    gain sampling. Lines go to console + Rust `smoke` listener. */
import { emit } from "@tauri-apps/api/event";
import { inTauri } from "@/lib/api";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { readBands } from "./bands";
import { engine } from "./engine";
import { getAnalyser, peekGraph } from "./graph";

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** StrictMode double-invokes effects — run the harness exactly once. */
let started = false;

const log = (msg: string) => {
  console.log(`[smoke] ${msg}`);
  if (inTauri) void emit("smoke", { msg });
};

async function waitTracks(timeoutMs: number): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (useLibraryStore.getState().tracks.length > 0) return true;
    await sleep(200);
  }
  return useLibraryStore.getState().tracks.length > 0;
}

export async function runSmokeIfEnabled(): Promise<void> {
  if (import.meta.env.VITE_ADIOS_SMOKE !== "1" || started) return;
  started = true;
  log("harness started");
  if (!(await waitTracks(15000))) {
    log("no tracks loaded within 15s - aborting");
    return;
  }
  const tracks = useLibraryStore.getState().tracks;
  log(`tracks loaded: ${tracks.length}`);

  usePlayerStore.getState().playQueue(
    tracks.map((t) => t.id),
    0,
  );
  await sleep(2500);
  const analyser = getAnalyser();
  const rms = analyser ? readBands(analyser).rms : -1;
  log(
    `rms=${rms.toFixed(4)} isPlaying=${usePlayerStore.getState().isPlaying} ` +
      `position=${usePlayerStore.getState().positionSec.toFixed(2)}s ` +
      `duration=${usePlayerStore.getState().durationSec.toFixed(2)}s`,
  );

  usePlayerStore.getState().seekTo(5);
  await sleep(700);
  log(`after seek(5): position=${usePlayerStore.getState().positionSec.toFixed(2)}s`);

  if (tracks.length > 1) {
    const before = usePlayerStore.getState().currentTrackId;
    usePlayerStore.getState().next();
    await sleep(1500);
    const after = usePlayerStore.getState();
    log(
      `after next: ${before} -> ${after.currentTrackId} ` +
        `position=${after.positionSec.toFixed(2)}s isPlaying=${after.isPlaying}`,
    );
    const beforePrev = after.currentTrackId;
    usePlayerStore.getState().prev();
    await sleep(1200);
    const back = usePlayerStore.getState();
    log(
      `after prev: ${beforePrev} -> ${back.currentTrackId} ` +
        `position=${back.positionSec.toFixed(2)}s isPlaying=${back.isPlaying}`,
    );
  } else {
    log("only one track - skipping next/prev test");
  }

  // Synthetic media-key event: exercises the mediaSession listener wiring
  // (the real RegisterHotKey path is verified separately via SendInput).
  if (inTauri && tracks.length > 1) {
    const beforeKey = usePlayerStore.getState().currentTrackId;
    await emit("media-key", "next");
    await sleep(900);
    const afterKey = usePlayerStore.getState();
    log(
      `after media-key(next): ${beforeKey} -> ${afterKey.currentTrackId} ` +
        `isPlaying=${afterKey.isPlaying}`,
    );
  }

  usePlayerStore.getState().setPlaying(false);
  await sleep(400);
  log(`final isPlaying=${usePlayerStore.getState().isPlaying} - smoke done`);

  if (import.meta.env.VITE_ADIOS_XFADE_TEST === "1") {
    await runCrossfadeTest();
  }
}

/** D3 crossfade listening test: objective gain/swap metrics (P3.4). */
async function runCrossfadeTest(): Promise<void> {
  const tracks = useLibraryStore.getState().tracks;
  if (tracks.length < 2) {
    log("xfade: need 2+ tracks - skipping");
    return;
  }
  usePlayerStore.getState().setCrossfadeMs(2000);
  usePlayerStore.getState().playQueue(tracks.map((t) => t.id), 0);
  log("xfade: crossfadeMs=2000, playing from start of queue");
  await sleep(1500);
  const duration = usePlayerStore.getState().durationSec;
  const target = Math.max(0, duration - 3.5);
  usePlayerStore.getState().seekTo(target);
  log(`xfade: seekTo ${target.toFixed(1)}s (fade starts at end-2s)`);

  const startId = usePlayerStore.getState().currentTrackId;
  let sawMidRange = false;
  for (let i = 0; i < 14; i++) {
    await sleep(400);
    const s = usePlayerStore.getState();
    const gains = engine.gainValues();
    if (gains) {
      const [a, b] = gains;
      if (a > 0.05 && a < 0.95 && b > 0.05) sawMidRange = true;
      log(
        `xfade sample ${i}: id=${s.currentTrackId} pos=${s.positionSec.toFixed(2)} ` +
          `gainActive=${a.toFixed(3)} gainInactive=${b.toFixed(3)}`,
      );
    } else {
      log(`xfade sample ${i}: id=${s.currentTrackId} pos=${s.positionSec.toFixed(2)} gains=n/a`);
    }
  }
  const end = usePlayerStore.getState();
  const graph = peekGraph();
  const finalGains = engine.gainValues();
  log(
    `xfade result: ${startId} -> ${end.currentTrackId} isPlaying=${end.isPlaying} ` +
      `midFadeGainsSeen=${sawMidRange} master=${graph?.master.gain.value.toFixed(2)} ` +
      `final=${finalGains ? finalGains.map((g) => g.toFixed(2)).join("/") : "n/a"}`,
  );
  usePlayerStore.getState().setPlaying(false);
  usePlayerStore.getState().setCrossfadeMs(0);
  log("xfade: done, crossfadeMs reset to 0 (D3 default OFF)");
}
