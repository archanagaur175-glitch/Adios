use crate::db::{queries, queries::NewTrack, queries::TrackRow, stream, DbState};
use tauri::State;

fn lock_err<E: std::fmt::Display>(err: E) -> String {
    format!("db lock poisoned: {err}")
}

#[tauri::command]
pub fn list_tracks(db: State<DbState>) -> Result<Vec<TrackRow>, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    queries::tracks_all(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_track(db: State<DbState>, id: i64) -> Result<Option<TrackRow>, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    queries::track_get(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn insert_track(db: State<DbState>, track: NewTrack) -> Result<i64, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    queries::track_insert(&conn, &track).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_track(db: State<DbState>, id: i64) -> Result<usize, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    queries::track_delete(&conn, id).map_err(|e| e.to_string())
}

/// P9.7: persist an online pick as a real 'stream' library row (dedupe by
/// `mbRecordingId` — re-pick refreshes the url). Returns the full row.
#[tauri::command]
pub fn add_stream_track(
    db: State<DbState>,
    url: String,
    artist: String,
    title: String,
    duration_ms: Option<i64>,
    mb_recording_id: Option<String>,
) -> Result<TrackRow, String> {
    let conn = db.0.lock().map_err(lock_err)?;
    stream::stream_track_upsert(
        &conn,
        url.trim(),
        artist.trim(),
        title.trim(),
        duration_ms,
        mb_recording_id.as_deref(),
    )
    .map_err(|e| e.to_string())
}
