//! Playlist JSON export/import (P8.2): versioned schema; tracks are matched
//! back into the library by STABLE identity (MB id → path → url), never by
//! row ids, so files survive copying between machines.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::playlist::{add_track, create, detail};

pub const EXPORT_VERSION: u32 = 1;

/// On-disk export shape (camelCase JSON; bump EXPORT_VERSION on breaking changes).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistExport {
    pub version: u32,
    pub name: String,
    pub tracks: Vec<TrackRef>,
}

/// Stable identity + display fields for one exported track.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackRef {
    pub path: Option<String>,
    pub url: Option<String>,
    pub mb_recording_id: Option<String>,
    pub artist: String,
    pub title: String,
    #[serde(default)]
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub playlist_id: i64,
    pub added: usize,
    pub missing: usize,
}

pub fn export(conn: &Connection, id: i64) -> rusqlite::Result<Option<PlaylistExport>> {
    let Some(detail) = detail(conn, id)? else {
        return Ok(None);
    };
    Ok(Some(PlaylistExport {
        version: EXPORT_VERSION,
        name: detail.name,
        tracks: detail
            .tracks
            .into_iter()
            .map(|t| TrackRef {
                path: t.path,
                url: t.url,
                mb_recording_id: t.mb_recording_id,
                artist: t.artist,
                title: t.title,
                duration_ms: t.duration_ms,
            })
            .collect(),
    }))
}

/// Create a playlist from an export; unresolvable refs are counted, never invented.
pub fn import(conn: &Connection, export: &PlaylistExport) -> rusqlite::Result<ImportResult> {
    let playlist_id = create(conn, &export.name)?;
    let mut added = 0;
    let mut missing = 0;
    for track in &export.tracks {
        match resolve_ref(conn, track)? {
            Some(track_id) => {
                add_track(conn, playlist_id, track_id)?;
                added += 1;
            }
            None => missing += 1,
        }
    }
    Ok(ImportResult { playlist_id, added, missing })
}

fn resolve_ref(conn: &Connection, track: &TrackRef) -> rusqlite::Result<Option<i64>> {
    if let Some(mb) = track.mb_recording_id.as_deref() {
        let id: Option<i64> = conn
            .query_row(
                "SELECT id FROM tracks WHERE mb_recording_id = ?1 LIMIT 1",
                [mb],
                |r| r.get(0),
            )
            .optional()?;
        if id.is_some() {
            return Ok(id);
        }
    }
    if let Some(path) = track.path.as_deref() {
        let id: Option<i64> = conn
            .query_row("SELECT id FROM tracks WHERE path = ?1 LIMIT 1", [path], |r| r.get(0))
            .optional()?;
        if id.is_some() {
            return Ok(id);
        }
    }
    if let Some(url) = track.url.as_deref() {
        return conn
            .query_row("SELECT id FROM tracks WHERE url = ?1 LIMIT 1", [url], |r| r.get(0))
            .optional();
    }
    Ok(None)
}
