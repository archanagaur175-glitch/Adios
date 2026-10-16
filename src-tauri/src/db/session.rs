//! Session queue persistence (P8.3): the m001 `queue` table stores the FE
//! play queue by track id + position. Loads re-densify positions (track
//! deletes cascade) and report how many rows were dropped so the FE can toast.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::queries::{row_to_track, settings_set, TrackRow, TRACK_COLUMNS};

pub const QUEUE_INDEX_KEY: &str = "sessionQueueIndex";

fn settings_read(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT value_json FROM settings WHERE key = ?1",
        [key],
        |r| r.get(0),
    )
    .optional()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueSnapshot {
    pub track_ids: Vec<i64>,
    pub queue_index: i64,
    /// Rows lost since the last save (deleted tracks); 0 when intact.
    pub dropped: i64,
}

/// Replace the stored queue and its current index (settings key).
pub fn save_queue(conn: &Connection, track_ids: &[i64], queue_index: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM queue", [])?;
    for (position, id) in track_ids.iter().enumerate() {
        conn.execute(
            "INSERT INTO queue (track_id, position) VALUES (?1, ?2)",
            params![id, position as i64],
        )?;
    }
    let index = if track_ids.is_empty() {
        -1
    } else {
        queue_index.clamp(0, track_ids.len() as i64 - 1)
    };
    settings_set(conn, QUEUE_INDEX_KEY, &index.to_string())
}

/// Dense load; self-heals sparse positions (cascade leftovers) in place.
pub fn load_queue(conn: &Connection) -> rusqlite::Result<QueueSnapshot> {
    let mut stmt = conn.prepare(
        "SELECT q.track_id, q.position FROM queue q
         JOIN tracks t ON t.id = q.track_id
         ORDER BY q.position ASC, q.track_id ASC",
    )?;
    let rows: Vec<(i64, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let max_pos_plus_one = rows.iter().map(|(_, p)| p + 1).max().unwrap_or(0);
    let dropped = (max_pos_plus_one - rows.len() as i64).max(0);
    let track_ids: Vec<i64> = rows.iter().map(|(id, _)| *id).collect();

    let saved_index: i64 = settings_read(conn, QUEUE_INDEX_KEY)
        .ok()
        .flatten()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(-1);
    let queue_index = if track_ids.is_empty() {
        -1
    } else {
        saved_index.clamp(0, track_ids.len() as i64 - 1)
    };

    // Self-heal: rewrite dense positions and the clamped index.
    save_queue(conn, &track_ids, queue_index)?;
    Ok(QueueSnapshot { track_ids, queue_index, dropped })
}

/// Remote tracks that have a disk-cache entry (offline-playable view).
pub fn cached_tracks(conn: &Connection) -> rusqlite::Result<Vec<TrackRow>> {
    let cols = TRACK_COLUMNS.replace(", ", ", t.");
    let sql = format!(
        "SELECT {cols} FROM tracks t
         WHERE t.url IS NOT NULL
           AND EXISTS (SELECT 1 FROM cache_index c WHERE c.url = t.url)
         ORDER BY t.artist, t.title, t.id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_track)?;
    rows.collect()
}
