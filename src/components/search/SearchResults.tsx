/** Grouped result rendering for the Ctrl+K overlay (P5.3/P9.5/P9.7):
    local tracks first ("local-first"), then online tracks (iTunes primary,
    MB fallback), albums, artists. Flattened row order matches the keyboard
    navigation index. Online track rows carry a "+ Add" button (P9.7). */

import { CoverArt } from "@/components/common/CoverArt";
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
  /** P9.5/P9.7: online track rows — `mbid` is "itunes:<id>" or an MB id. */
  mbid?: string;
  /** P9.7: iTunes id when the row came from the iTunes picker. */
  itunesId?: number;
  durationMs?: number;
  /** P12 covers: local cover path or iTunes artwork URL (or neither). */
  coverPath?: string | null;
  artworkUrl?: string;
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
  /** "iTunes" when tracks came from iTunes; "MusicBrainz tracks" otherwise. */
  trackLabel: string;
}

export function buildGroups(input: GroupInput): ResultGroup[] {
  const groups: ResultGroup[] = [];

  const local = input.localResults.map((t) => ({
    key: `local-${t.id}`,
    title: t.title,
    sub: [t.artist, t.album].filter(Boolean).join(" · ") || undefined,
    right: formatDuration(t.durationMs),
    trackId: t.id,
    coverPath: t.coverPath,
  }));
  if (local.length > 0) groups.push({ label: "In your library", rows: local });

  const trackRows = input.onlineTracks.map((r, i) => ({
    key: `tracks-${r.itunesId ?? r.mbid ?? i}`,
    title: r.title,
    sub: r.artist || undefined,
    right: formatDuration(r.durationMs),
    mbid: r.mbid,
    itunesId: r.itunesId,
    durationMs: r.durationMs,
    artworkUrl: r.artworkUrl,
  }));
  if (trackRows.length > 0) groups.push({ label: input.trackLabel, rows: trackRows });

  const addMeta = (label: string, items: OnlineResult[]) => {
    const rows = items.map((r, i) => ({
      key: `${label}-${r.mbid || i}`,
      title: r.title,
      sub: r.artist || undefined,
    }));
    if (rows.length > 0) groups.push({ label, rows });
  };
  addMeta("MusicBrainz albums", input.onlineAlbums);
  addMeta("MusicBrainz artists", input.onlineArtists);

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
  /** P9.7: "… added" state for a row just saved to a playlist. */
  addedKey?: string | null;
  onHover: (index: number) => void;
  onPick: (row: ResultRow) => void;
  /** P9.7/P9.8: "+ Add" on online track rows; the button element anchors
      the playlist menu (portal — the dialog clips absolute children). */
  onAdd?: (row: ResultRow, button: HTMLElement) => void;
  /** P12: "⭳ Download" on online track rows (pin to the disk cache). */
  onDownload?: (row: ResultRow) => void;
}

export function SearchResultsList({
  groups,
  activeIndex,
  pickingKey,
  addedKey,
  onHover,
  onPick,
  onAdd,
  onDownload,
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
                const isOnlineTrack = row.mbid != null && onAdd != null;
                return (
                  <li
                    key={row.key}
                    onMouseMove={() => onHover(index)}
                    onClick={() => onPick(row)}
                    className={`flex cursor-pointer items-center gap-2 rounded-card px-3 py-2 transition ${
                      index === activeIndex ? "bg-glass" : ""
                    }`}
                  >
                    <CoverArt track={row} size={32} />
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-14 text-ink-1">{row.title}</p>
                      {row.sub && (
                        <p className="truncate text-12 text-ink-3">{row.sub}</p>
                      )}
                    </div>
                    {pickingKey === row.key ? (
                      <span className="text-12 text-accent">Resolving…</span>
                    ) : addedKey === row.key ? (
                      <span className="text-12 text-accent">Added ✓</span>
                    ) : (
                      row.right && (
                        <span className="text-12 tabular text-ink-3">{row.right}</span>
                      )
                    )}
                    {isOnlineTrack && (
                      <>
                        <button
                          type="button"
                          aria-label={`Download “${row.title}”`}
                          onClick={(e) => {
                            e.stopPropagation();
                            onDownload?.(row);
                          }}
                          className="rounded-6 px-1.5 py-0.5 text-14 text-ink-3 transition hover:bg-glass hover:text-ink-1"
                        >
                          ⭳
                        </button>
                        <button
                          type="button"
                          aria-label={`Add “${row.title}” to a playlist`}
                          onClick={(e) => {
                            e.stopPropagation();
                            onAdd?.(row, e.currentTarget);
                          }}
                          className="rounded-6 px-1.5 py-0.5 text-14 text-ink-3 transition hover:bg-glass hover:text-ink-1"
                        >
                          +
                        </button>
                      </>
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
