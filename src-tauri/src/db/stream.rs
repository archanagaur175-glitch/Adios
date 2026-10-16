//! P9.7: persistent online ("stream") tracks — iTunes/MB picks
//! saved as real library rows (source 'stream', url set) so they survive
//! restarts, become FTS-searchable (insert triggers), and satisfy the
//! playlist_tracks FK. Dedupe by mb_recording_id (re-pick refreshes url).

use rusqlite::{params, Connection};

use super::queries::{row_to_track, track_get, TrackRow, TRACK_COLUMNS};

/// Insert (or refresh) a stream track; returns the full row.
pub fn stream_track_upsert(
    conn: &Connection,
    url: &str,
    artist: &str,
    title: &str,
    duration_ms: Option<i64>,
    mb_recording_id: Option<&str>,
) -> rusqlite::Result<TrackRow> {
    if let Some(mid) = mb_recording_id {
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM tracks WHERE mb_recording_id = ?1 LIMIT 1",
                [mid],
                |r| r.get(0),
            )
            .ok();
        if let Some(id) = existing {
            conn.execute(
                "UPDATE tracks SET url = ?2, artist = ?3, title = ?4, duration_ms = ?5,
                    updated_at = datetime('now')
                 WHERE id = ?1",
                params![id, url, artist, title, duration_ms],
            )?;
            return track_get(conn, id)?.ok_or_else(|| {
                rusqlite::Error::QueryReturnedNoRows
            });
        }
    }
    conn.execute(
        "INSERT INTO tracks (source, path, url, artist, title, duration_ms, mb_recording_id)
         VALUES ('stream', NULL, ?1, ?2, ?3, ?4, ?5)",
        params![url, artist, title, duration_ms, mb_recording_id],
    )?;
    let id = conn.last_insert_rowid();
    let sql = format!("SELECT {TRACK_COLUMNS} FROM tracks WHERE id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    stmt.query_row([id], row_to_track)
}
