//! Search commands (P5.1/P5.3): `search_local` is the instant FTS5 layer of
//! the Ctrl+K overlay; `search_metadata` is the MusicBrainz layer that
//! appends after the 275 ms debounce (local-first, both cache-first).

use serde_json::to_value;
use tauri::State;

use crate::db::DbState;
use crate::meta::mb_client::{MbClient, ReqwestTransport};
use crate::meta::mb_types::{MbArtist, MbRecording, MbRelease, SearchKind};

/// Instant local library search (FTS5 prefix match, ranked, ≤ 50 rows).
#[tauri::command]
pub fn search_local(db: State<DbState>, query: String) -> Result<Vec<crate::db::queries::TrackRow>, String> {
    let conn = db
        .0
        .lock()
        .map_err(|e| format!("db lock poisoned: {e}"))?;
    crate::db::fts::search(&conn, &query, 50).map_err(|e| format!("search failed: {e}"))
}

/// Search MusicBrainz through the cache-first client.
/// `kind`: "artist" | "recording" | "release".
#[tauri::command]
pub async fn search_metadata(
    client: State<'_, MbClient<ReqwestTransport>>,
    kind: String,
    query: String,
) -> Result<serde_json::Value, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("empty query".to_owned());
    }
    let result: Result<serde_json::Value, String> = match kind.as_str() {
        "artist" => to_value(client.search::<MbArtist>(SearchKind::Artist, q).await?)
            .map_err(|e| e.to_string()),
        "recording" => to_value(client.search::<MbRecording>(SearchKind::Recording, q).await?)
            .map_err(|e| e.to_string()),
        "release" => to_value(client.search::<MbRelease>(SearchKind::Release, q).await?)
            .map_err(|e| e.to_string()),
        other => Err(format!("unknown search kind: {other}")),
    };
    result
}
