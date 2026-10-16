//! Playlist access (P8.2): CRUD over `playlists` + `playlist_tracks` with
//! dense `position` ordering. JSON export/import lives in `playlist_io.rs`.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::queries::{row_to_track, TrackRow, TRACK_COLUMNS};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSummary {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub tracks: Vec<TrackRow>,
}

pub fn create(conn: &Connection, name: &str) -> rusqlite::Result<i64> {
    conn.execute("INSERT INTO playlists (name) VALUES (?1)", [name])?;
    Ok(conn.last_insert_rowid())
}

pub fn rename(conn: &Connection, id: i64, name: &str) -> rusqlite::Result<bool> {
    Ok(conn.execute("UPDATE playlists SET name = ?1 WHERE id = ?2", params![name, id])? > 0)
}

pub fn delete(conn: &Connection, id: i64) -> rusqlite::Result<bool> {
    // Members cascade via the FK (m001).
    Ok(conn.execute("DELETE FROM playlists WHERE id = ?1", [id])? > 0)
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<PlaylistSummary>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.created_at, p.updated_at,
                (SELECT COUNT(*) FROM playlist_tracks pt WHERE pt.playlist_id = p.id)
         FROM playlists p ORDER BY p.updated_at DESC, p.id DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(PlaylistSummary {
            id: r.get(0)?,
            name: r.get(1)?,
            created_at: r.get(2)?,
            updated_at: r.get(3)?,
            track_count: r.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn detail(conn: &Connection, id: i64) -> rusqlite::Result<Option<PlaylistDetail>> {
    let head = conn
        .query_row(
            "SELECT id, name, created_at, updated_at FROM playlists WHERE id = ?1",
            [id],
            |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
            },
        )
        .optional()?;
    let Some((id, name, created_at, updated_at)) = head else {
        return Ok(None);
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT t.{cols} FROM playlist_tracks pt JOIN tracks t ON t.id = pt.track_id
         WHERE pt.playlist_id = ?1 ORDER BY pt.position ASC",
        cols = TRACK_COLUMNS.replace(", ", ", t.")
    ))?;
    let tracks = stmt
        .query_map([id], row_to_track)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Some(PlaylistDetail { id, name, created_at, updated_at, tracks }))
}

/// Append a track (false when the playlist is missing or already has it).
pub fn add_track(conn: &Connection, playlist_id: i64, track_id: i64) -> rusqlite::Result<bool> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM playlists WHERE id = ?1",
        [playlist_id],
        |r| r.get(0),
    )?;
    if exists == 0 {
        return Ok(false);
    }
    let dup: i64 = conn.query_row(
        "SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
        params![playlist_id, track_id],
        |r| r.get(0),
    )?;
    if dup > 0 {
        return Ok(false);
    }
    conn.execute(
        "INSERT INTO playlist_tracks (playlist_id, track_id, position)
         VALUES (?1, ?2, COALESCE((SELECT MAX(position) + 1 FROM playlist_tracks
                                   WHERE playlist_id = ?1), 0))",
        params![playlist_id, track_id],
    )?;
    touch(conn, playlist_id)?;
    Ok(true)
}

/// Remove a member and renumber the rest densely.
pub fn remove_track(conn: &Connection, playlist_id: i64, track_id: i64) -> rusqlite::Result<bool> {
    let gone = conn.execute(
        "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
        params![playlist_id, track_id],
    )?;
    if gone == 0 {
        return Ok(false);
    }
    renumber(conn, playlist_id)?;
    touch(conn, playlist_id)?;
    Ok(true)
}

/// Persist a full new order (dense 0..n). False when `track_ids` is not
/// exactly the playlist's current membership — never half-applies.
pub fn reorder(conn: &Connection, playlist_id: i64, track_ids: &[i64]) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position ASC",
    )?;
    let current: Vec<i64> = stmt
        .query_map([playlist_id], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let same_set = current.len() == track_ids.len()
        && current.iter().all(|id| track_ids.contains(id));
    if !same_set {
        return Ok(false);
    }
    for (i, id) in track_ids.iter().enumerate() {
        conn.execute(
            "UPDATE playlist_tracks SET position = ?1 WHERE playlist_id = ?2 AND track_id = ?3",
            params![i as i64, playlist_id, id],
        )?;
    }
    touch(conn, playlist_id)?;
    Ok(true)
}

fn renumber(conn: &Connection, playlist_id: i64) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT track_id FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position ASC",
    )?;
    let ids: Vec<i64> = stmt
        .query_map([playlist_id], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for (i, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE playlist_tracks SET position = ?1 WHERE playlist_id = ?2 AND track_id = ?3",
            params![i as i64, playlist_id, id],
        )?;
    }
    Ok(())
}

fn touch(conn: &Connection, playlist_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE playlists SET updated_at = datetime('now') WHERE id = ?1",
        [playlist_id],
    )?;
    Ok(())
}
