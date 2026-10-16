/** Grouped result rendering for the Ctrl+K overlay (P5.3):
    local tracks first ("local-first"), then MusicBrainz tracks/albums/artists.
    Flattened row order matches the keyboard navigation index. */

import { formatDuration } from "@/lib/format";
import type { OnlineResult } from "@/store/searchStore";
import type { Track } from "@/lib/types";

export interface ResultRow {
  key: string;
  title: string;
  sub?: string;
  right?: string;
  /** Present for local rows — picking one starts playback. */
  trackId?: number;
  /** P9.5: present for MusicBrainz track rows — picking one resolves+streams. */
  mbid?: string;
  durationMs?: number;
}

export interface ResultGroup {
  label: string;
  rows: ResultRow[];
}

export interface GroupInput {
  localResults: Track[];
  onlineTracks: OnlineResult[];
  onlineAlbums: OnlineResult[];
  onlineArtists: OnlineResult[];
}

export function buildGroups(input: GroupInput): ResultGroup[] {
  const groups: ResultGroup[] = [];

  const local = input.localResults.map((t) => ({
    key: `local-${t.id}`,
    title: t.title,
    sub: [t.artist, t.album].filter(Boolean).join(" · ") || undefined,
    right: formatDuration(t.durationMs),
    trackId: t.id,
  }));
  if (local.length > 0) groups.push({ label: "In your library", rows: local });

  const addMb = (
    label: string,
    items: OnlineResult[],
    withDuration: boolean,
    withMbid = false,
  ) => {
    const rows = items.map((r, i) => ({
      key: `${label}-${r.mbid || i}`,
      title: r.title,
      sub: r.artist || undefined,
      right: withDuration ? formatDuration(r.durationMs) : undefined,
      mbid: withMbid ? r.mbid : undefined,
      durationMs: withDuration ? r.durationMs : undefined,
    }));
    if (rows.length > 0) groups.push({ label, rows });
  };
  addMb("MusicBrainz tracks", input.onlineTracks, true, true);
  addMb("MusicBrainz albums", input.onlineAlbums, false);
  addMb("MusicBrainz artists", input.onlineArtists, false);

  return groups;
}

/** Flatten groups in render order — the overlay's arrow-key list. */
export function flattenGroups(groups: ResultGroup[]): ResultRow[] {
  return groups.flatMap((group) => group.rows);
}

interface Props {
  groups: ResultGroup[];
  activeIndex: number;
  /** P9.5: key of the row currently resolving through the stream chain. */
  pickingKey?: string | null;
  onHover: (index: number) => void;
  onPick: (row: ResultRow) => void;
}

export function SearchResultsList({
  groups,
  activeIndex,
  pickingKey,
  onHover,
  onPick,
}: Props) {
  let offset = 0;
  return (
    <>
      {groups.map((group) => {
        const start = offset;
        offset += group.rows.length;
        return (
          <section key={group.label} className="mt-2">
            <h4 className="px-3 py-1 text-11 font-medium uppercase tracking-wide text-ink-3">
              {group.label}
            </h4>
            <ul>
              {group.rows.map((row, i) => {
                const index = start + i;
                return (
                  <li
                    key={row.key}
                    onMouseMove={() => onHover(index)}
                    onClick={() => onPick(row)}
                    className={`flex cursor-pointer items-center gap-3 rounded-card px-3 py-2 transition ${
                      index === activeIndex ? "bg-glass" : ""
                    }`}
                  >
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-14 text-ink-1">{row.title}</p>
                      {row.sub && (
                        <p className="truncate text-12 text-ink-3">{row.sub}</p>
                      )}
                    </div>
                    {pickingKey === row.key ? (
                      <span className="text-12 text-accent">Resolving…</span>
                    ) : (
                      row.right && (
                        <span className="text-12 tabular text-ink-3">{row.right}</span>
                      )
                    )}
                  </li>
                );
              })}
            </ul>
          </section>
        );
      })}
    </>
  );
}
