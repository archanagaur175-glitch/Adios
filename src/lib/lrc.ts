/** LRC lyric parser (P7.2, DESIGN §7): timestamps `[mm:ss.xx]` / `.xxx`,
    multi-tag lines, `[offset:±ms]` header, malformed lines skipped — no
    throws. Pure module: unit-tested in lrc.test.ts (Rule: offsets, multi-
    line, malformed skipped). */

/** One timestamped lyric entry. `timeMs` already includes the offset. */
export interface LrcLine {
  timeMs: number;
  text: string;
}

/** Leading tag groups at line start: `[00:12.00]`, `[offset:+500]`, `[ar:x]`. */
const TAG_RE = /^\[([^\]]*)\]/;
/** `[m+]mm:ss(.frac)` — frac 1 digit = tenths, 2 = centis, 3 = millis. */
const TIME_RE = /^(\d{1,3}):(\d{1,2})(?:[.:](\d{1,3}))?$/;
const OFFSET_RE = /^offset:\s*([+-]?\d+)\s*$/i;

function tagToMs(m: string, s: string, frac?: string): number | null {
  const sec = Number(s);
  if (!Number.isFinite(sec) || sec > 59) return null;
  const ms = frac ? Number(frac.padEnd(3, "0")) : 0;
  return ((Number(m) * 60) + sec) * 1000 + ms;
}

/** Parse LRC text → lines sorted by time (stable), offset applied and
    clamped at 0. Unparsable lines/metadata tags are skipped. */
export function parseLrc(text: string): LrcLine[] {
  let offsetMs = 0;
  const lines: LrcLine[] = [];
  for (const raw of text.split(/\r?\n/)) {
    let rest = raw.trim();
    if (!rest) continue;
    const times: number[] = [];
    while (rest.startsWith("[")) {
      const m = TAG_RE.exec(rest);
      if (!m) break;
      const inner = m[1];
      rest = rest.slice(m[0].length);
      const off = OFFSET_RE.exec(inner);
      if (off) {
        offsetMs = Number(off[1]);
        continue;
      }
      const t = TIME_RE.exec(inner);
      const ms = t ? tagToMs(t[1], t[2], t[3]) : null;
      if (ms !== null) times.push(ms);
    }
    if (times.length === 0) {
      // Metadata (`[ar:…]`), garbage tags, or tag-less prose → skip.
      continue;
    }
    const textPart = rest.trim();
    for (const t of times) lines.push({ timeMs: Math.max(0, t + offsetMs), text: textPart });
  }
  lines.sort((a, b) => a.timeMs - b.timeMs); // stable in ES2019+
  return lines;
}

/** Index of the last line at or before `timeMs`; -1 before the first. */
export function activeLineIndex(lines: LrcLine[], timeMs: number): number {
  let lo = 0;
  let hi = lines.length - 1;
  let ans = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].timeMs <= timeMs) {
      ans = mid;
      lo = mid + 1;
    } else hi = mid - 1;
  }
  return ans;
}
