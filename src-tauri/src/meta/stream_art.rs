//! P13: iTunes artwork persistence for stream picks. The search API hands
//! us a 100×100 URL; the same asset exists at 600×600 (URL size segment
//! swap, verified live in gate131). Fetch → 512² JPEG → palette →
//! `covers/stream-{track_id}.jpg` + cover columns on the row, so every
//! DB-reading surface (player, Home, playlists) gets art like local tracks.

use std::path::Path;
use std::time::Instant;

use rusqlite::params;

use super::cover_art::{self, CoverArtClient};
use super::palette;
use crate::resolver::SharedDb;

/// iTunes artwork URLs embed the pixel size (`…/100x100bb.jpg`); the
/// 600 px variant is the same asset at higher resolution.
pub fn upscaled_url(url: &str) -> String {
    url.replace("100x100bb", "600x600bb")
}

/// Persist a stream pick's cover + palette on the row (no MB release id —
/// iTunes art comes from the search API, not CAA).
fn set_cover(
    conn: &rusqlite::Connection,
    id: i64,
    cover_path: &str,
    accent: &str,
    accent2: &str,
    colors_json: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE tracks SET cover_path = ?2, accent = ?3, accent2 = ?4,
            dominant_colors_json = ?5, updated_at = datetime('now')
         WHERE id = ?1",
        params![id, cover_path, accent, accent2, colors_json],
    )
}

/// Fetch `artwork_url` (600 px first, original on failure), shape it, and
/// cache art + palette on the track row. Returns the cover path.
pub async fn fetch_and_cache(
    client: &CoverArtClient,
    db: &SharedDb,
    covers_dir: &Path,
    track_id: i64,
    artwork_url: &str,
) -> Result<String, String> {
    let started = Instant::now();
    let url = artwork_url.trim();
    let big = upscaled_url(url);
    let (raw, source) = match client.fetch_bytes(&big).await {
        Ok(b) => (b, big),
        Err(_) => (client.fetch_bytes(url).await?, url.to_owned()),
    };
    let shaped = cover_art::encode_cover_jpeg(&raw)?;
    let pal = palette::extract(&shaped)?;
    std::fs::create_dir_all(covers_dir).map_err(|e| format!("covers dir failed: {e}"))?;
    let cover_file = covers_dir.join(format!("stream-{track_id}.jpg"));
    std::fs::write(&cover_file, &shaped).map_err(|e| format!("write cover failed: {e}"))?;
    let cover_path = cover_file.to_string_lossy().into_owned();
    let colors_json =
        serde_json::to_string(&pal.colors).map_err(|e| format!("palette json: {e}"))?;
    {
        let conn = db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        set_cover(&conn, track_id, &cover_path, &pal.accent, &pal.accent2, &colors_json)
            .map_err(|e| format!("db write failed: {e}"))?;
    }
    let ms = started.elapsed().as_millis() as i64;
    println!(
        "[stream-art] fetched track={track_id} src={source} raw={}B → {}B accent={} in {ms}ms",
        raw.len(),
        shaped.len(),
        pal.accent
    );
    Ok(cover_path)
}
