/** play() plus ONE data-ready retry.
    Android WebView can reject play() right after a src swap while the new
    load has no data yet and the session's media engagement is still low —
    even though the store intent is "playing" (gate182: Next left the queue
    Paused and a second tap was needed). The caller already reverts to an
    honest paused state when play() rejects, so the retry is safe: it either
    starts once data arrives, or rejects again and the same fallback runs.
    Bounded by a timeout so a stalled load can never hang the state. */

/** Wait ceiling for the retry window before attempting play() anyway. */
const RETRY_WAIT_MS = 2500;

function whenPlayable(el: HTMLMediaElement): Promise<void> {
  return new Promise((resolve) => {
    const done = () => {
      clearTimeout(timer);
      el.removeEventListener("canplay", done);
      resolve();
    };
    const timer: ReturnType<typeof setTimeout> = setTimeout(done, RETRY_WAIT_MS);
    el.addEventListener("canplay", done);
  });
}

export async function playWithRetry(el: HTMLMediaElement): Promise<void> {
  try {
    await el.play();
    return;
  } catch (error) {
    // 3 = HAVE_FUTURE_DATA (literal: HTMLMediaElement statics are absent
    // in node test env). Data already there → not a load-window refusal.
    if (el.readyState >= 3) throw error;
    await whenPlayable(el);
    await el.play();
  }
}
