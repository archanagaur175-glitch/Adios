/** Pure play-tracking gate (P10): one record per track START. A deliberate
    start counts — playQueue (bumps playEpoch, even for the same track) and
    any currentTrackId change while playing (next, crossfade swap). Session
    restore, pause, seek, and resume of the same track do not. No imports —
    unit-tested in isolation (see playGate.test.ts). */

export interface PlayGateState {
  currentTrackId: number | null;
  isPlaying: boolean;
  playEpoch: number;
}

export function shouldRecordPlay(prev: PlayGateState, next: PlayGateState): boolean {
  if (!next.isPlaying || next.currentTrackId === null) return false;
  return (
    next.playEpoch !== prev.playEpoch || next.currentTrackId !== prev.currentTrackId
  );
}
