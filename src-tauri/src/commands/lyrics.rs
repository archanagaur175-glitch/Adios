//! Lyrics command (P7.1): cache-first LRCLIB fetch for one track row.
//! The synced panel / seek wiring is P7.2 — this step only lands the
//! client + cache + command + FE api binding.

use tauri::State;

use crate::db::DbState;
use crate::meta::lrclib::{LyricsClient, ReqwestTransport};
use crate::meta::lrclib_types::{LyricsPayload, LyricsQuery};

/// Fetch lyrics for `track_id` (local-first: `lyrics_cache` hit/negative
/// returns with zero network). Returns `{found:false,…}` when known absent;
/// `Err` only for transient failures (nothing cached on those).
#[tauri::command]
pub async fn get_lyrics(
    client: State<'_, LyricsClient<ReqwestTransport>>,
    db: State<'_, DbState>,
    track_id: i64,
) -> Result<LyricsPayload, String> {
    let track = {
        let conn = db
            .0
            .lock()
            .map_err(|e| format!("db lock poisoned: {e}"))?;
        crate::db::queries::track_get(&conn, track_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("track {track_id} not found"))?
    };
    if track.artist.trim().is_empty() || track.title.trim().is_empty() {
        return Err("track has no artist/title to query".to_owned());
    }
    let query = LyricsQuery {
        artist: track.artist,
        title: track.title,
        album: track.album,
        duration_ms: track.duration_ms,
    };
    match client.fetch(&query).await? {
        Some(payload) => Ok(payload),
        None => Ok(LyricsPayload::not_found()),
    }
}
