//! Enrichment row access (P5.4): tracks still awaiting an MB match and the
//! two identifier columns written when a match is accepted. Decoupled from
//! `queries.rs` so each file stays single-purpose.

use rusqlite::{params, Connection, Result};

/// Minimal projection enrichment needs — deliberately NOT `TrackRow`.
#[derive(Debug, Clone)]
pub struct UnmatchedTrack {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub duration_ms: Option<i64>,
}

/// Tracks with no MB recording match yet, oldest first, capped.
pub fn unmatched(conn: &Connection, limit: i64) -> Result<Vec<UnmatchedTrack>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, artist, duration_ms FROM tracks
         WHERE mb_recording_id IS NULL ORDER BY id LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |r| {
        Ok(UnmatchedTrack {
            id: r.get(0)?,
            title: r.get(1)?,
            artist: r.get(2)?,
            duration_ms: r.get(3)?,
        })
    })?;
    rows.collect()
}

/// Persist the accepted MusicBrainz identifiers on a track row.
pub fn set_mb_ids(
    conn: &Connection,
    id: i64,
    recording_id: &str,
    release_id: Option<&str>,
) -> Result<usize> {
    conn.execute(
        "UPDATE tracks SET mb_recording_id = ?2, mb_release_id = COALESCE(?3, mb_release_id),
            updated_at = datetime('now')
         WHERE id = ?1",
        params![id, recording_id, release_id],
    )
}
