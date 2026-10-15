use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------- Settings ----------

/// Read all settings as a key → raw JSON string map.
pub fn settings_get_all(conn: &Connection) -> Result<HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT key, value_json FROM settings ORDER BY key")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

/// Insert or update one setting. `value_json` must be valid JSON text.
pub fn settings_set(conn: &Connection, key: &str, value_json: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value_json) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
        params![key, value_json],
    )?;
    Ok(())
}

// ---------- Tracks ----------

/// Input for inserting a track. Serialized from the webview as camelCase.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTrack {
    pub source: String,
    pub path: Option<String>,
    pub url: Option<String>,
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub year: Option<i64>,
    pub track_no: Option<i64>,
    pub disc_no: Option<i64>,
    pub file_size: Option<i64>,
    pub mtime: Option<i64>,
}

/// Track row returned to the webview (camelCase to match `src/lib/types.ts`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRow {
    pub id: i64,
    pub source: String,
    pub path: Option<String>,
    pub url: Option<String>,
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub year: Option<i64>,
    pub track_no: Option<i64>,
    pub disc_no: Option<i64>,
    pub cover_path: Option<String>,
    pub accent: Option<String>,
    pub accent2: Option<String>,
    pub dominant_colors_json: Option<String>,
    pub added_at: String,
    pub play_count: i64,
}

const TRACK_COLUMNS: &str = "id, source, path, url, artist, title, album, duration_ms,
    year, track_no, disc_no, cover_path, accent, accent2, dominant_colors_json,
    added_at, play_count";

fn row_to_track(row: &rusqlite::Row<'_>) -> Result<TrackRow> {
    Ok(TrackRow {
        id: row.get(0)?,
        source: row.get(1)?,
        path: row.get(2)?,
        url: row.get(3)?,
        artist: row.get(4)?,
        title: row.get(5)?,
        album: row.get(6)?,
        duration_ms: row.get(7)?,
        year: row.get(8)?,
        track_no: row.get(9)?,
        disc_no: row.get(10)?,
        cover_path: row.get(11)?,
        accent: row.get(12)?,
        accent2: row.get(13)?,
        dominant_colors_json: row.get(14)?,
        added_at: row.get(15)?,
        play_count: row.get(16)?,
    })
}

/// Insert a track; returns the new id. Re-inserting the same path is a no-op.
pub fn track_insert(conn: &Connection, track: &NewTrack) -> Result<i64> {
    conn.execute(
        "INSERT INTO tracks (source, path, url, artist, title, album, duration_ms,
            year, track_no, disc_no, file_size, mtime)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(path) DO NOTHING",
        params![
            track.source,
            track.path,
            track.url,
            track.artist,
            track.title,
            track.album,
            track.duration_ms,
            track.year,
            track.track_no,
            track.disc_no,
            track.file_size,
            track.mtime,
        ],
    )?;
    if let Some(path) = &track.path {
        let mut stmt = conn.prepare("SELECT id FROM tracks WHERE path = ?1")?;
        return stmt.query_row([path], |row| row.get(0));
    }
    Ok(conn.last_insert_rowid())
}

/// True when a track with this file path already exists.
pub fn track_exists(conn: &Connection, path: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM tracks WHERE path = ?1")?;
    let mut rows = stmt.query_map([path], |_| Ok(()))?;
    Ok(rows.next().is_some())
}

/// Set the cover file path for a track (written by the folder scan / CAA fetch).
pub fn track_set_cover(conn: &Connection, id: i64, cover_path: &str) -> Result<usize> {
    conn.execute(
        "UPDATE tracks SET cover_path = ?2 WHERE id = ?1",
        params![id, cover_path],
    )
}

pub fn track_get(conn: &Connection, id: i64) -> Result<Option<TrackRow>> {
    let sql = format!("SELECT {TRACK_COLUMNS} FROM tracks WHERE id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map([id], row_to_track)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

/// All tracks ordered by artist, album, track number.
pub fn tracks_all(conn: &Connection) -> Result<Vec<TrackRow>> {
    let sql = format!(
        "SELECT {TRACK_COLUMNS} FROM tracks
         ORDER BY artist COLLATE NOCASE, album COLLATE NOCASE, track_no, title"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_track)?;
    rows.collect()
}

pub fn track_delete(conn: &Connection, id: i64) -> Result<usize> {
    conn.execute("DELETE FROM tracks WHERE id = ?1", [id])
}
