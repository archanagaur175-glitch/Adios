//! Playlist commands (P8.2): CRUD + reorder + versioned JSON file
//! export/import (Rust does the file IO — the webview has no fs plugin).

use crate::db::playlist;
use crate::db::playlist_io::{self, PlaylistExport};
use crate::db::DbState;
use tauri::State;

fn lock_err<E: std::fmt::Display>(err: E) -> String {
    format!("db lock poisoned: {err}")
}

#[tauri::command]
pub fn playlist_create(db: State<DbState>, name: String) -> Result<i64, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    playlist::create(&conn, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn playlist_rename(db: State<DbState>, id: i64, name: String) -> Result<bool, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    playlist::rename(&conn, id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn playlist_delete(db: State<DbState>, id: i64) -> Result<bool, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    playlist::delete(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn playlist_list(db: State<DbState>) -> Result<Vec<playlist::PlaylistSummary>, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    playlist::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn playlist_detail(db: State<DbState>, id: i64) -> Result<Option<playlist::PlaylistDetail>, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    playlist::detail(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn playlist_add_track(db: State<DbState>, playlist_id: i64, track_id: i64) -> Result<bool, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    playlist::add_track(&conn, playlist_id, track_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn playlist_remove_track(db: State<DbState>, playlist_id: i64, track_id: i64) -> Result<bool, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    playlist::remove_track(&conn, playlist_id, track_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn playlist_reorder(db: State<DbState>, playlist_id: i64, track_ids: Vec<i64>) -> Result<bool, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    playlist::reorder(&conn, playlist_id, &track_ids).map_err(|e| e.to_string())
}

/// Write the playlist as pretty JSON to `path` (save-dialog target).
#[tauri::command]
pub fn playlist_export_file(db: State<DbState>, id: i64, path: String) -> Result<usize, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    let dump = playlist_io::export(&conn, id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("playlist {id} not found"))?;
    let text = serde_json::to_string_pretty(&dump).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("write {path}: {e}"))?;
    Ok(dump.tracks.len())
}

/// Read a playlist export from `path` and create the playlist (version-guarded).
#[tauri::command]
pub fn playlist_import_file(db: State<DbState>, path: String) -> Result<playlist_io::ImportResult, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    let dump: PlaylistExport = serde_json::from_str(&text).map_err(|e| format!("parse {path}: {e}"))?;
    if dump.version != playlist_io::EXPORT_VERSION {
        return Err(format!(
            "unsupported export version {} (this app reads version {})",
            dump.version,
            playlist_io::EXPORT_VERSION
        ));
    }
    let conn = db.0.lock().map_err(lock_err)?;
    playlist_io::import(&conn, &dump).map_err(|e| e.to_string())
}
