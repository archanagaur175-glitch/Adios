//! Remote proxy streaming (P4.3 / architecture §3.2 + P4.4 re-resolve).
//! Remote URL → reqwest → client; full responses are tee'd to the disk
//! cache via temp→rename and indexed in `cache_index`; client Range
//! requests forward upstream and are never cached (partial files).
//!
//! P9.4 (OQ3): googlevideo throttles Range-less GETs with 403 — every
//! upstream request now carries a Range header. Full-file fetches use
//! `Range: bytes=0-`; a 206 whose range starts at byte 0 IS the whole
//! body, so it is tee'd into the cache and served to the client as 200.
//!
//! P4.4: upstream 401/403/404/410 (expired googlevideo URL) triggers
//! exactly ONE re-resolve through the chain (`reresolve.rs`) before the
//! status is passed through.

use std::path::PathBuf;

use axum::body::Body;
use axum::http::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::Response;
use bytes::Bytes;

use crate::server::handlers::ServerState;
use crate::server::reresolve::{re_resolve_once, ReResolved};
use crate::server::tee::{tee_body, tee_open};

/// Statuses that mean "expired/forbidden upstream URL" (re-resolve once).
const EXPIRED_STATUSES: [u16; 4] = [401, 403, 404, 410];

/// Serve a remote URL: disk-cache fast path, else stream upstream (tee on full).
pub async fn remote_response(state: &ServerState, url: &str, range_header: Option<&str>) -> Response {
    // Fast path: fully cached already.
    let cached = state
        .db
        .lock()
        .ok()
        .and_then(|conn| crate::server::cache::cache_index_get(&conn, url));
    if let Some(row) = cached {
        let path = PathBuf::from(&row.local_path);
        if tokio::fs::metadata(&path).await.is_ok() {
            if let Ok(conn) = state.db.lock() {
                let _ = crate::server::cache::cache_index_touch(&conn, url);
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

    // OQ3: never send a Range-less GET upstream — synthesize `bytes=0-`
    // for full-file fetches (client Range headers pass through unchanged).
    let req = state
        .client
        .get(url)
        .header(axum::http::header::RANGE, range_header.unwrap_or("bytes=0-"));
    let upstream = match req.send().await {
        Ok(res) => res,
        Err(err) => return plain(StatusCode::BAD_GATEWAY, format!("upstream error: {err}")),
    };
    let status = upstream.status();
    if !status.is_success() {
        if EXPIRED_STATUSES.contains(&status.as_u16()) {
            match re_resolve_once(state, url).await {
                Some(ReResolved::Url(new_url)) if new_url != url => {
                    println!("[proxy] {url} -> {new_url} (single re-resolve)");
                    return Box::pin(remote_response(state, &new_url, range_header)).await;
                }
                Some(ReResolved::Path(path)) => {
                    println!("[proxy] {url} -> local {} (single re-resolve)", path.display());
                    return crate::server::handlers::path_response(&path, range_header).await;
                }
                _ => {}
            }
        }
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

    // OQ3: a client full GET (no Range) was sent upstream as `bytes=0-`.
    // A 206 whose range starts at byte 0 carries the whole body — treat it
    // as a 200 and still tee it into the cache. A 206 starting elsewhere
    // (misbehaving server) is streamed through un-cached, unconverted.
    let full_from_zero = range_header.is_none()
        && status == StatusCode::PARTIAL_CONTENT
        && content_range
            .as_ref()
            .and_then(|v| v.to_str().ok())
            .is_some_and(|s| s.starts_with("bytes 0-"));
    let tee_ok = range_header.is_none() && (status == StatusCode::OK || full_from_zero);

    let body = if tee_ok {
        let tee = tee_open(&state.cache_dir, state.db.clone(), url, &mime, upstream, content_length)
            .await;
        tee_body(tee)
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
    *res.status_mut() = if full_from_zero {
        StatusCode::OK
    } else {
        status
    };
    let h = res.headers_mut();
    h.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    h.insert(CONTENT_TYPE, mime_header);
    if let Some(len) = content_length {
        h.insert(CONTENT_LENGTH, HeaderValue::from_str(&len.to_string()).unwrap());
    }
    // Content-Range is only meaningful to clients that asked for a Range.
    if range_header.is_some() {
        if let Some(cr) = content_range {
            h.insert(CONTENT_RANGE, cr);
        }
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
