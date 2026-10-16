//! Play-history commands (P10): record a play, list the History tab.

use crate::db::history;
use crate::db::{queries::TrackRow, DbState};
use tauri::State;

fn lock_err<E: std::fmt::Display>(err: E) -> String {
    format!("db lock poisoned: {err}")
}

/// Called by the FE once per track start (`lib/playHistory.ts` guard).
#[tauri::command]
pub fn record_play(db: State<DbState>, id: i64) -> Result<usize, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    history::record_play(&conn, id).map_err(|e| e.to_string())
}

/// History tab: most-recently-played tracks (play_count + last_played_at
/// ride the standard TrackRow).
#[tauri::command]
pub fn list_play_history(db: State<DbState>) -> Result<Vec<TrackRow>, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    history::play_history(&conn, 200).map_err(|e| e.to_string())
}
