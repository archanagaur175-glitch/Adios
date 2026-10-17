import { useMemo } from "react";
import {
  decadeTracks,
  forgottenFavorites,
  onRepeat,
  topArtists,
  topDecades,
  tracksByArtist,
} from "@/lib/taste";
import { useLibraryStore } from "@/store/libraryStore";
import { TrackShelf } from "@/components/home/TrackShelf";
import type { Track } from "@/lib/types";

/** P16 section C: local taste shelves — computed in-memory from play
    history (zero network); every shelf hides itself when it has nothing
    worth showing (fresh install → no shelves). */

interface Shelf {
  id: string;
  title: string;
  subtitle?: string;
  tracks: Track[];
}

const SHELF_SIZE = 8;
/** "Because you listened to" needs a real mini-catalog, not one song. */
const ARTIST_MIN_TRACKS = 2;

function decadeTitle(decade: number): string {
  return `The ${decade % 100}s mix`;
}

export function TasteShelves() {
  const tracks = useLibraryStore((s) => s.tracks);
  const shelves = useMemo(() => {
    const now = Date.now();
    const out: Shelf[] = [];

    const repeat = onRepeat(tracks, now);
    if (repeat.length > 0) out.push({ id: "on-repeat", title: "On repeat", tracks: repeat });

    const artist = topArtists(tracks, now, 1)[0];
    if (artist) {
      const theirs = tracksByArtist(tracks, artist.artist, now).slice(0, SHELF_SIZE);
      if (theirs.length >= ARTIST_MIN_TRACKS) {
        out.push({
          id: `because-${artist.artist.toLowerCase()}`,
          title: "Because you listened to",
          subtitle: artist.artist,
          tracks: theirs,
        });
      }
    }

    const forgotten = forgottenFavorites(tracks, now);
    if (forgotten.length > 0) {
      out.push({ id: "forgotten", title: "Forgotten favorites", tracks: forgotten });
    }

    for (const decade of topDecades(tracks, now)) {
      const list = decadeTracks(tracks, decade, now);
      if (list.length > 0) {
        out.push({ id: `decade-${decade}`, title: decadeTitle(decade), tracks: list });
      }
    }
    return out;
  }, [tracks]);

  return (
    <>
      {shelves.map((shelf) => (
        <TrackShelf
          key={shelf.id}
          title={shelf.title}
          subtitle={shelf.subtitle}
          tracks={shelf.tracks}
        />
      ))}
    </>
  );
}
