//! Remote proxy streaming (P4.3 / architecture §3.2).
//! Remote URL → reqwest → client; full responses are tee'd to the disk cache
//! via temp→rename and indexed in `cache_index`; Range requests forward
//! upstream and are never cached (partial files).

use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::Response;
use bytes::Bytes;

use crate::resolver::SharedDb;
use crate::server::cache::{cache_index_get, cache_index_put, cache_index_touch, cache_path_for_url};

/// Serve a remote URL: disk-cache fast path, else stream upstream (tee on full).
pub async fn remote_response(
    db: &SharedDb,
    cache_dir: &Path,
    client: &reqwest::Client,
    url: &str,
    range_header: Option<&str>,
) -> Response {
    // Fast path: fully cached already.
    let cached = db.lock().ok().and_then(|conn| cache_index_get(&conn, url));
    if let Some(row) = cached {
        let path = PathBuf::from(&row.local_path);
        if tokio::fs::metadata(&path).await.is_ok() {
            if let Ok(conn) = db.lock() {
                let _ = cache_index_touch(&conn, url);
            }
            let len = row.bytes as u64;
            return match crate::server::range::parse_range(range_header, len) {
                Ok(Some(range)) => {
                    crate::server::handlers::file_response(&path, len, Some(range)).await
                }
                Ok(None) => crate::server::handlers::file_response(&path, len, None).await,
                Err(()) => range_not_satisfiable(len),
            };
        }
    }

    let mut req = client.get(url);
    if let Some(range) = range_header {
        req = req.header(axum::http::header::RANGE, range);
    }
    let upstream = match req.send().await {
        Ok(res) => res,
        Err(err) => return plain(StatusCode::BAD_GATEWAY, format!("upstream error: {err}")),
    };
    let status = upstream.status();
    if !status.is_success() {
        return plain(status, format!("upstream status {status}"));
    }

    let mime = upstream
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_owned();
    let mime_header = HeaderValue::from_str(&mime)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    let content_length = upstream.content_length();
    let content_range = upstream.headers().get(CONTENT_RANGE).cloned();

    let body = if range_header.is_none() {
        // The cache dir may not exist yet (fresh install).
        let _ = tokio::fs::create_dir_all(cache_dir).await;
        let temp = cache_dir.join(format!(
            ".part-{}-{}",
            crate::resolver::fnv1a_hex(url),
            uuid::Uuid::new_v4()
        ));
        let final_path = cache_path_for_url(cache_dir, url, &mime);
        let file = tokio::fs::File::create(&temp).await.ok();
        let tee = Tee {
            resp: upstream,
            file,
            temp,
            final_path,
            db: db.clone(),
            url: url.to_owned(),
            mime,
            written: 0,
            expected: content_length,
        };
        Body::from_stream(futures::stream::unfold(tee, step_tee))
    } else {
        let stream = futures::stream::unfold(upstream, |mut resp| async move {
            match resp.chunk().await {
                Ok(Some(chunk)) => Some((Ok::<Bytes, std::io::Error>(chunk), resp)),
                Ok(None) => None,
                Err(_) => None,
            }
        });
        Body::from_stream(stream)
    };

    let mut res = Response::new(body);
    *res.status_mut() = status;
    let h = res.headers_mut();
    h.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    h.insert(CONTENT_TYPE, mime_header);
    if let Some(len) = content_length {
        h.insert(CONTENT_LENGTH, HeaderValue::from_str(&len.to_string()).unwrap());
    }
    if let Some(cr) = content_range {
        h.insert(CONTENT_RANGE, cr);
    }
    res
}

fn plain(status: StatusCode, message: String) -> Response {
    let mut res = Response::new(Body::from(message));
    *res.status_mut() = status;
    res
}

fn range_not_satisfiable(len: u64) -> Response {
    let mut res = plain(StatusCode::RANGE_NOT_SATISFIABLE, "bad range".to_string());
    res.headers_mut().insert(
        CONTENT_RANGE,
        HeaderValue::from_str(&format!("bytes */{len}")).unwrap(),
    );
    res
}

// ---------- tee: client body + disk cache ----------

struct Tee {
    resp: reqwest::Response,
    file: Option<tokio::fs::File>,
    temp: PathBuf,
    final_path: PathBuf,
    db: SharedDb,
    url: String,
    mime: String,
    written: u64,
    expected: Option<u64>,
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
        let _ = cache_index_put(&conn, &tee.url, &path, Some(&tee.mime), tee.written);
    }
}
