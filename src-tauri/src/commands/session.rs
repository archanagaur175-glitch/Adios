//! Session commands (P8.3): persist/restore the play queue and list
//! disk-cached tracks (the offline "Cached" view).

use crate::db::session::{self, QueueSnapshot};
use crate::db::{queries::TrackRow, DbState};
use tauri::State;

fn lock_err<E: std::fmt::Display>(err: E) -> String {
    format!("db lock poisoned: {err}")
}

/// Replace the stored queue (track ids in order) + current index.
#[tauri::command]
pub fn session_save_queue(
    db: State<DbState>,
    track_ids: Vec<i64>,
    queue_index: i64,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(lock_err)?;
    session::save_queue(&conn, &track_ids, queue_index).map_err(|e| e.to_string())
}

/// Dense queue snapshot; self-heals sparse rows and clamped index.
#[tauri::command]
pub fn session_load_queue(db: State<DbState>) -> Result<QueueSnapshot, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    session::load_queue(&conn).map_err(|e| e.to_string())
}

/// Remote tracks with a disk-cache entry (offline-playable).
#[tauri::command]
pub fn list_cached_tracks(db: State<DbState>) -> Result<Vec<TrackRow>, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    session::cached_tracks(&conn).map_err(|e| e.to_string())
}
