//! Full-GET disk-cache writer (P4.3 / architecture §3.2): streams the
//! upstream body to the client while teeing it to `.part-uuid` → rename
//! → `cache_index` insert. Range requests never reach this path.

use std::path::PathBuf;

use axum::body::Body;
use bytes::Bytes;

use crate::resolver::SharedDb;
use crate::server::cache::{cache_index_put, cache_path_for_url};

pub struct Tee {
    pub resp: reqwest::Response,
    pub file: Option<tokio::fs::File>,
    pub temp: PathBuf,
    pub final_path: PathBuf,
    pub db: SharedDb,
    pub url: String,
    pub mime: String,
    pub written: u64,
    pub expected: Option<u64>,
}

/// Streaming body wrapper: `Body::from_stream(unfold(tee, step_tee))`.
pub fn tee_body(tee: Tee) -> Body {
    Body::from_stream(futures::stream::unfold(tee, step_tee))
}

async fn step_tee(mut tee: Tee) -> Option<(Result<Bytes, std::io::Error>, Tee)> {
    match tee.resp.chunk().await {
        Ok(Some(chunk)) => {
            if let Some(file) = tee.file.as_mut() {
                if tokio::io::AsyncWriteExt::write_all(file, &chunk)
                    .await
                    .is_err()
                {
                    tee.file = None; // disk full etc. — keep streaming to client
                }
            }
            tee.written += chunk.len() as u64;
            // Finalize as soon as the last expected byte lands: the client
            // resolves at content-length, before the stream's final None poll.
            if tee.expected.is_some_and(|exp| tee.written >= exp) {
                finalize_tee(&mut tee).await;
            }
            Some((Ok(chunk), tee))
        }
        Ok(None) => {
            finalize_tee(&mut tee).await;
            None
        }
        Err(_) => {
            // Upstream died mid-stream: drop the partial temp file.
            if let Some(mut file) = tee.file.take() {
                let _ = tokio::io::AsyncWriteExt::shutdown(&mut file).await;
            }
            let _ = tokio::fs::remove_file(&tee.temp).await;
            None
        }
    }
}

/// Complete a full download: flush, temp→rename, index in `cache_index`.
/// Idempotent — the file handle is taken on the first call.
async fn finalize_tee(tee: &mut Tee) {
    let Some(mut file) = tee.file.take() else {
        return;
    };
    let _ = tokio::io::AsyncWriteExt::flush(&mut file).await;
    let _ = tokio::io::AsyncWriteExt::shutdown(&mut file).await;
    if tokio::fs::rename(&tee.temp, &tee.final_path).await.is_err() {
        let _ = tokio::fs::remove_file(&tee.temp).await;
        return;
    }
    // One INSERT — negligible cost; keep it inline so the index row exists
    // as soon as the rename lands (no spawn_blocking race with readers).
    if let Ok(conn) = tee.db.lock() {
        let path = tee.final_path.to_string_lossy();
        if cache_index_put(&conn, &tee.url, &path, Some(&tee.mime), tee.written).is_ok() {
            // P8.1: enforce the size cap on every write (quiet when it fits).
            let cap = crate::server::evict::cache_cap_bytes(&conn);
            let dir = tee.final_path.parent().unwrap_or(std::path::Path::new("."));
            if let Ok(stats) = crate::server::evict::evict_to_cap(&conn, dir, cap)
                && stats.removed + stats.removed_parts + stats.removed_orphans > 0
            {
                println!(
                    "[cache] evict (tee) removed={} parts={} orphans={} left={}B",
                    stats.removed, stats.removed_parts, stats.removed_orphans,
                    stats.remaining_bytes
                );
            }
        }
    }
}

/// Open the tee: ensures the cache dir exists, creates `.part-uuid`.
pub async fn tee_open(
    cache_dir: &std::path::Path,
    db: SharedDb,
    url: &str,
    mime: &str,
    resp: reqwest::Response,
    expected: Option<u64>,
) -> Tee {
    let _ = tokio::fs::create_dir_all(cache_dir).await;
    let temp = cache_dir.join(format!(
        ".part-{}-{}",
        crate::resolver::fnv1a_hex(url),
        uuid::Uuid::new_v4()
    ));
    let file = tokio::fs::File::create(&temp).await.ok();
    Tee {
        final_path: cache_path_for_url(cache_dir, url, mime),
        resp,
        file,
        temp,
        db,
        url: url.to_owned(),
        mime: mime.to_owned(),
        written: 0,
        expected,
    }
}
