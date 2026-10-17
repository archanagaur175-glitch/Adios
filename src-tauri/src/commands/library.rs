use crate::db::{queries, queries::NewTrack, queries::TrackRow, stream, DbState};
use crate::meta::cover_art::CoverArtClient;
use crate::meta::stream_art;
use tauri::{AppHandle, Manager, State};

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

/// P9.7 persist an online pick as a real 'stream' library row (dedupe by
/// `mbRecordingId` — re-pick refreshes the url). P13: when the pick
/// carries an iTunes artwork URL and the row has no cover yet, fetch it
/// best-effort (art failure never fails the add). Returns the full row.
#[tauri::command]
pub async fn add_stream_track(
    app: AppHandle,
    client: State<'_, CoverArtClient>,
    db: State<'_, DbState>,
    url: String,
    artist: String,
    title: String,
    duration_ms: Option<i64>,
    mb_recording_id: Option<String>,
    artwork_url: Option<String>,
) -> Result<TrackRow, String> {
    let row = {
        let conn = db.0.lock().map_err(lock_err)?;
        stream::stream_track_upsert(
            &conn,
            url.trim(),
            artist.trim(),
            title.trim(),
            duration_ms,
            mb_recording_id.as_deref(),
        )
        .map_err(|e| e.to_string())?
    };
    if let Some(art) = artwork_url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
        if row.cover_path.is_none() {
            let covers_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?
                .join("covers");
            match stream_art::fetch_and_cache(&client, &db.shared(), &covers_dir, row.id, art)
                .await
            {
                Ok(path) => {
                    println!("[stream-art] track={} cover={}", row.id, path);
                    let conn = db.0.lock().map_err(lock_err)?;
                    if let Some(updated) =
                        queries::track_get(&conn, row.id).map_err(|e| e.to_string())?
                    {
                        return Ok(updated);
                    }
                }
                Err(e) => println!("[stream-art] track={} failed: {e}", row.id),
            }
        }
    }
    Ok(row)
}
