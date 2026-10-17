-- Migration 003: download pins (P12). Pinned cache rows survive LRU
-- eviction — a user-chosen download stays until explicitly deleted or
-- its file vanishes. Default 0 keeps every existing row evictable.
ALTER TABLE cache_index ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
