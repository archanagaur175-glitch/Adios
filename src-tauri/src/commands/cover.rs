//! Cover fetch command (P5.2): thin wrapper over `meta::cover_store` (the
//! shared persistence path also driven by import enrichment, P5.4).

use tauri::{AppHandle, Manager, State};

use crate::db::DbState;
use crate::meta::cover_art::CoverArtClient;
use crate::meta::cover_store::{self, FetchCoverResult};

/// Fetch the front cover for `release_mbid` and cache art + palette on the
/// track row. `track_id` must exist; `release_mbid` is any valid MBID.
#[tauri::command]
pub async fn fetch_cover(
    app: AppHandle,
    client: State<'_, CoverArtClient>,
    db: State<'_, DbState>,
    track_id: i64,
    release_mbid: String,
) -> Result<FetchCoverResult, String> {
    let covers_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    cover_store::fetch_and_cache(
        &client,
        &db.shared(),
        &covers_dir,
        track_id,
        release_mbid.trim(),
    )
    .await
}
