-- Migration 004: track genre (P16 taste/mood shelves). Local rows get their
-- tag genre on the next scan (backfill only when NULL, see track_set_genre);
-- iTunes picks carry primaryGenreName straight from the search response.
ALTER TABLE tracks ADD COLUMN genre TEXT;
