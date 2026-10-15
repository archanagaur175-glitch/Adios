//! Metadata search command (P5.1): exposes the MB client so the P5 gate
//! ("repeat query served from meta_cache with NO network hit — logged") can
//! be verified live. The local-first Ctrl+K UI lands in 5.3 and consumes
//! this same command.
use serde_json::to_value;
use tauri::State;

use crate::meta::mb_client::{MbClient, ReqwestTransport};
use crate::meta::mb_types::{MbArtist, MbRecording, MbRelease, SearchKind};

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
