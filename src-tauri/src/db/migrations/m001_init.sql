-- Migration 001: core schema (docs/ARCHITECTURE.md §3.1)

CREATE TABLE IF NOT EXISTS tracks (
  id INTEGER PRIMARY KEY,
  path TEXT UNIQUE,
  source TEXT NOT NULL,
  url TEXT,
  artist TEXT NOT NULL DEFAULT '',
  title TEXT NOT NULL DEFAULT '',
  album TEXT,
  duration_ms INTEGER,
  year INTEGER,
  track_no INTEGER,
  disc_no INTEGER,
  mb_recording_id TEXT,
  mb_release_id TEXT,
  cover_path TEXT,
  accent TEXT,
  accent2 TEXT,
  dominant_colors_json TEXT,
  file_size INTEGER,
  mtime INTEGER,
  added_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  play_count INTEGER NOT NULL DEFAULT 0,
  last_played_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks (artist);
CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks (album);

-- FTS5 external-content table over tracks, kept in sync by triggers.
CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5 (
  artist,
  title,
  album,
  content = 'tracks',
  content_rowid = 'id',
  tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS tracks_ai AFTER INSERT ON tracks BEGIN
  INSERT INTO tracks_fts (rowid, artist, title, album)
  VALUES (new.id, new.artist, new.title, new.album);
END;

CREATE TRIGGER IF NOT EXISTS tracks_ad AFTER DELETE ON tracks BEGIN
  INSERT INTO tracks_fts (tracks_fts, rowid, artist, title, album)
  VALUES ('delete', old.id, old.artist, old.title, old.album);
END;

CREATE TRIGGER IF NOT EXISTS tracks_au AFTER UPDATE ON tracks BEGIN
  INSERT INTO tracks_fts (tracks_fts, rowid, artist, title, album)
  VALUES ('delete', old.id, old.artist, old.title, old.album);
  INSERT INTO tracks_fts (rowid, artist, title, album)
  VALUES (new.id, new.artist, new.title, new.album);
END;

CREATE TABLE IF NOT EXISTS playlists (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS playlist_tracks (
  playlist_id INTEGER NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
  track_id INTEGER NOT NULL REFERENCES tracks (id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  PRIMARY KEY (playlist_id, track_id)
);

CREATE TABLE IF NOT EXISTS queue (
  track_id INTEGER PRIMARY KEY REFERENCES tracks (id) ON DELETE CASCADE,
  position INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS cache_index (
  url TEXT PRIMARY KEY,
  local_path TEXT NOT NULL,
  mime TEXT,
  bytes INTEGER NOT NULL DEFAULT 0,
  expires_at TEXT,
  last_access TEXT NOT NULL DEFAULT (datetime('now')),
  hit_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS lyrics_cache (
  track_key TEXT PRIMARY KEY,
  synced_json TEXT,
  plain_text TEXT,
  fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS meta_cache (
  cache_key TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL
);
