//! CAA front-cover persistence (P5.2 core, extracted in P5.4 so import
//! enrichment and the `fetch_cover` command share ONE code path): cache
//! short-circuit (stored MBID + file on disk = zero network) → fetch →
//! 512² JPEG shape → 3-color palette → track row. Logs `[caa] cache hit`
//! / `[caa] fetched …` exactly as the P5.2 gate recorded.

use std::path::Path;
use std::time::Instant;

use rusqlite::OptionalExtension;
use serde::Serialize;

use super::cover_art::{self, CoverArtClient};
use super::palette;
use crate::db::queries;
use crate::resolver::SharedDb;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchCoverResult {
    pub cover_path: String,
    pub accent: String,
    pub accent2: String,
    pub colors: Vec<String>,
    pub cached: bool,
    pub ms: i64,
}

/// Fetch the front cover for `release_mbid` (already trimmed/lowercased by
/// callers) and cache art + palette + release MBID on the track row.
/// `track_id` must exist. Never errors on a missing row's cover dir — the
/// caller creates `covers/` up front.
pub async fn fetch_and_cache(
    client: &CoverArtClient,
    db: &SharedDb,
    covers_dir: &Path,
    track_id: i64,
    release_mbid: &str,
) -> Result<FetchCoverResult, String> {
    let started = Instant::now();
    let mbid = release_mbid.trim().to_lowercase();

    type Row = (Option<String>, Option<String>, Option<String>, Option<String>, Option<String>);
    let row: Option<Row> = {
        let conn = db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        conn.query_row(
            "SELECT mb_release_id, cover_path, accent, accent2, dominant_colors_json
             FROM tracks WHERE id = ?1",
            [track_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()
        .map_err(|e| format!("db read failed: {e}"))?
    };
    let (stored_mbid, stored_cover, stored_accent, stored_accent2, stored_colors) =
        row.ok_or_else(|| format!("track {track_id} not found"))?;

    let file_name = format!("caa-{mbid}.jpg");
    let on_disk = |p: &Option<String>| {
        p.as_deref()
            .is_some_and(|p| p.ends_with(&file_name) && Path::new(p).is_file())
    };
    if stored_mbid.as_deref() == Some(mbid.as_str()) && on_disk(&stored_cover) {
        let cover_path = stored_cover.unwrap_or_default();
        let colors = stored_colors
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
            .unwrap_or_default();
        println!("[caa] cache hit track={track_id} cover={cover_path}");
        return Ok(FetchCoverResult {
            cover_path,
            accent: stored_accent.unwrap_or_default(),
            accent2: stored_accent2.unwrap_or_default(),
            colors,
            cached: true,
            ms: started.elapsed().as_millis() as i64,
        });
    }

    std::fs::create_dir_all(covers_dir)
        .map_err(|e| format!("caa: covers dir failed: {e}"))?;
    let (raw, source) = client.fetch_front(&mbid).await?;
    let shaped = cover_art::encode_cover_jpeg(&raw)?;
    let pal = palette::extract(&shaped)?;
    let cover_file = covers_dir.join(&file_name);
    std::fs::write(&cover_file, &shaped)
        .map_err(|e| format!("caa: write cover failed: {e}"))?;
    let cover_path = cover_file.to_string_lossy().into_owned();
    let colors_json =
        serde_json::to_string(&pal.colors).map_err(|e| format!("palette json: {e}"))?;
    {
        let conn = db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        queries::track_set_cover_palette(
            &conn,
            track_id,
            &cover_path,
            &pal.accent,
            &pal.accent2,
            &colors_json,
            &mbid,
        )
        .map_err(|e| format!("db write failed: {e}"))?;
    }
    let ms = started.elapsed().as_millis() as i64;
    println!(
        "[caa] fetched mbid={mbid} src={source} raw={}B → {}B accent={} in {ms}ms",
        raw.len(),
        shaped.len(),
        pal.accent
    );
    Ok(FetchCoverResult {
        cover_path,
        accent: pal.accent,
        accent2: pal.accent2,
        colors: pal.colors.to_vec(),
        cached: false,
        ms,
    })
}
