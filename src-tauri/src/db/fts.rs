//! Local FTS5 search (P5.3): user text → sanitized `MATCH` pattern → ranked
//! track rows. The overlay queries this first and instantly; MusicBrainz
//! results append afterwards (local-first).

use rusqlite::{Connection, Result};

use super::queries::{row_to_track, TrackRow, TRACK_COLUMNS};

/// Prefix pattern for FTS5: each whitespace token becomes a quoted term
/// with `*`, so operators (`AND`/`OR`/`:`/`(`…) the user types are searched
/// literally instead of parsed. None for empty input.
pub fn match_pattern(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|token| format!("\"{}\"*", token.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

/// Ranked local search (bm25 via `ORDER BY rank`), newest-style relevance
/// first, capped at `limit`. Empty/whitespace input returns no rows.
pub fn search(conn: &Connection, query: &str, limit: i64) -> Result<Vec<TrackRow>> {
    let Some(pattern) = match_pattern(query) else {
        return Ok(Vec::new());
    };
    // tracks_fts exposes the same column names (external content) — prefix
    // every projection with `tracks.` to keep the join unambiguous.
    let cols = TRACK_COLUMNS
        .split(',')
        .map(|c| format!("tracks.{}", c.trim()))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT {cols} FROM tracks_fts
         JOIN tracks ON tracks.id = tracks_fts.rowid
         WHERE tracks_fts MATCH ?1 ORDER BY rank LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params![pattern, limit], row_to_track)?;
    rows.collect()
}
