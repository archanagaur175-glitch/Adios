-- Migration 002: lyrics_cache gains negative-row support (P7.1).
-- m001 created the table (track_key PK, synced_json, plain_text, fetched_at);
-- nothing has ever written to it, but per D14 applied migrations are never
-- edited — extend instead. status: 'hit' | 'not_found' (404 / instrumental:
-- a row that exists so the network is never re-hit for the same query).

ALTER TABLE lyrics_cache ADD COLUMN status TEXT NOT NULL DEFAULT 'hit';
ALTER TABLE lyrics_cache ADD COLUMN lrclib_id INTEGER;
ALTER TABLE lyrics_cache ADD COLUMN instrumental INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_lyrics_cache_status ON lyrics_cache (status);
