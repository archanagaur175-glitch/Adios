//! Pinned downloads (P12): `download_track` fully fetches a stream track
//! into the disk cache and pins the row (survives LRU eviction per m003).
//! OQ3 full-GET (`Range: bytes=0-`) + one re-resolve on expired URLs.
//! Success adopts the downloaded URL as `tracks.url` (badge/Cached/proxy
//! joins) and writes the resolution back (next resolve = local file).

use rusqlite::{params, OptionalExtension};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{queries, DbState};
use crate::resolver::cache::{invalidate_resolution, put_resolution};
use crate::resolver::{cache as rcache, Query, SharedChain, SharedTable};
use crate::server::cache as scache;
use crate::server::evict;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadDone {
    pub track_id: i64,
    pub url: String,
    pub bytes: u64,
    pub already_cached: bool,
}

async fn emit_progress(app: &AppHandle, track_id: i64, received: u64, total: Option<u64>) {
    let _ = app.emit(
        "download-progress",
        serde_json::json!({ "trackId": track_id, "received": received, "total": total }),
    );
}

async fn fetch_to_cache(
    app: &AppHandle,
    cache_dir: &std::path::Path,
    url: &str,
    track_id: i64,
) -> Result<(u64, String), String> {
    emit_progress(app, track_id, 0, None).await;
    let res = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::RANGE, "bytes=0-")
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    let status = res.status();
    if !status.is_success() {
        return Err(format!("download failed: HTTP {status}"));
    }
    let total = res.content_length();
    let mime = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_owned();
    let temp = cache_dir.join(format!(
        ".part-dl-{}-{}",
        crate::resolver::fnv1a_hex(url),
        uuid::Uuid::new_v4()
    ));
    let mut file = tokio::fs::File::create(&temp)
        .await
        .map_err(|e| format!("create temp file: {e}"))?;
    let mut stream = res.bytes_stream();
    let mut written: u64 = 0;
    use futures::StreamExt;
    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("download stream: {e}"))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| format!("write temp file: {e}"))?;
        written += chunk.len() as u64;
        emit_progress(app, track_id, written, total).await;
    }
    drop(file);
    if written == 0 {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err("download was empty".to_owned());
    }
    let final_path = scache::cache_path_for_url(cache_dir, url, &mime);
    if tokio::fs::rename(&temp, &final_path).await.is_err() {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err("finalize download failed".to_owned());
    }
    emit_progress(app, track_id, written, total).await;
    Ok((written, mime))
}

fn is_expired(err: &str) -> bool {
    ["401", "403", "404", "410"].iter().any(|code| err.contains(code))
}

/// Pin the index row for `url` (file must exist) and adopt it as the
/// track's stream URL — badges/Cached/proxy joins all use tracks.url.
fn pin_adopt(db: &DbState, track_id: i64, url: &str) -> Result<Option<i64>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let Some(row) = scache::cache_index_get(&conn, url) else {
        return Ok(None);
    };
    if std::fs::metadata(&row.local_path).is_err() {
        return Ok(None);
    }
    scache::cache_index_pin(&conn, url).map_err(|e| e.to_string())?;
    conn.execute("UPDATE tracks SET url = ?2 WHERE id = ?1", params![track_id, url]).map_err(|e| e.to_string())?;
    Ok(Some(row.bytes))
}

/// Download one stream track to the cache and pin it. Local tracks error;
/// an already-cached URL just gets pinned. Expired googlevideo URLs get
/// ONE re-resolve (invalidate → chain → retry) like the proxy.
#[tauri::command]
pub async fn download_track(
    app: AppHandle,
    db: State<'_, DbState>,
    chain: State<'_, SharedChain>,
    table: State<'_, SharedTable>,
    track_id: i64,
) -> Result<DownloadDone, String> {
    let track = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        queries::track_get(&conn, track_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("track {track_id} not found"))?
    };
    if track.source == "local" {
        return Err("local files are already on disk".to_owned());
    }
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let cache_dir = data_dir.join("cache");
    tokio::fs::create_dir_all(&cache_dir).await.map_err(|e| format!("create cache dir: {e}"))?;
    let query = Query {
        artist: track.artist.clone(),
        title: track.title.clone(),
        album: track.album.clone(),
        duration_ms: track.duration_ms,
    };
    let mut candidate = chain.resolve(&query).await.map_err(|e| e.to_string())?;
    // Chain cache hit (local path): pin + adopt the indexed URL.
    if !rcache::is_remote_url(&candidate.url_or_path) {
        let indexed: Option<String> = {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            conn.query_row(
                "SELECT url FROM cache_index WHERE local_path = ?1",
                [&candidate.url_or_path],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
        };
        let bytes = match indexed.as_deref() {
            Some(u) => pin_adopt(&db, track_id, u)?,
            None => None,
        };
        return Ok(DownloadDone {
            track_id,
            url: indexed.unwrap_or_default(),
            bytes: bytes.unwrap_or(0) as u64,
            already_cached: bytes.is_some(),
        });
    }
    let mut url = candidate.url_or_path.clone();
    let mut retried = false;
    loop {
        if let Some(bytes) = pin_adopt(&db, track_id, &url)? {
            return Ok(DownloadDone { track_id, url, bytes: bytes as u64, already_cached: true });
        }
        match fetch_to_cache(&app, &cache_dir, &url, track_id).await {
            Ok((written, mime)) => {
                let path =
                    scache::cache_path_for_url(&cache_dir, &url, &mime).to_string_lossy().to_string();
                let conn = db.0.lock().map_err(|e| e.to_string())?;
                scache::cache_index_put(&conn, &url, &path, Some(&mime), written)
                    .map_err(|e| e.to_string())?;
                scache::cache_index_pin(&conn, &url).map_err(|e| e.to_string())?;
                conn.execute("UPDATE tracks SET url = ?2 WHERE id = ?1", params![track_id, &url]).map_err(|e| e.to_string())?;
                let cap = evict::cache_cap_bytes(&conn);
                if let Ok(stats) = evict::evict_to_cap(&conn, &cache_dir, cap) {
                    println!("[cache] evict (download) removed={} left={}B", stats.removed, stats.remaining_bytes);
                }
                drop(conn);
                let _ = put_resolution(&db.shared(), &query, &candidate);
                table.register(&url, &query);
                println!("[download] track {track_id} pinned {written} B");
                return Ok(DownloadDone { track_id, url, bytes: written, already_cached: false });
            }
            Err(err) if is_expired(&err) && !retried => {
                println!("[download] {url} expired ({err}) — single re-resolve");
                retried = true;
                let _ = invalidate_resolution(&db.shared(), &query);
                candidate = chain.resolve(&query).await.map_err(|e| e.to_string())?;
                if !rcache::is_remote_url(&candidate.url_or_path) {
                    return Err("re-resolve returned a local path unexpectedly".to_owned());
                }
                url = candidate.url_or_path.clone();
            }
            Err(err) => return Err(err),
        }
    }
}
