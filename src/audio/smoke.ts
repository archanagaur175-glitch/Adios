/** Debug-only verification harness (P3.3 gate): enabled with VITE_ADIOS_SMOKE=1.
    Plays the first library track, logs AnalyserNode RMS > 0 while playing,
    then exercises seek → next → pause. Every line goes to console + the
    Rust `smoke` event listener (printed as [smoke] ... in the dev terminal). */
import { emit } from "@tauri-apps/api/event";
import { inTauri } from "@/lib/api";
import { useLibraryStore } from "@/store/libraryStore";
import { usePlayerStore } from "@/store/playerStore";
import { readBands } from "./bands";
import { getAnalyser } from "./graph";

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
    log("no tracks loaded within 15s — aborting");
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

  usePlayerStore.getState().setPlaying(false);
  await sleep(400);
  log(`final isPlaying=${usePlayerStore.getState().isPlaying} - smoke done`);
}
