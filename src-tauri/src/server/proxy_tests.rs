//! P4.3 tests: cache_index bookkeeping + remote proxy streaming.

use super::cache;
use super::test_util::{test_start, temp_file};

/// googlevideo-style upstream: 403 unless the request carries a Range
/// header (OQ3 — throttling detection punishes Range-less full GETs).
async fn throttle_by_range(headers: axum::http::HeaderMap) -> axum::response::Response {
    use axum::response::IntoResponse;
    const LEN: usize = 2048;
    if !headers.contains_key(axum::http::header::RANGE) {
        return (axum::http::StatusCode::FORBIDDEN, "no Range").into_response();
    }
    axum::response::Response::builder()
        .status(axum::http::StatusCode::PARTIAL_CONTENT)
        .header("content-type", "audio/mp4")
        .header("content-range", format!("bytes 0-{}/{}", LEN - 1, LEN))
        .header("content-length", LEN)
        .body(axum::body::Body::from(vec![7u8; LEN]))
        .unwrap()
}

/// OQ3/P9.4: full-file fetches must send `Range: bytes=0-` upstream, a
/// 206-from-zero is served to the client as 200, and the body is still
/// tee'd into the disk cache. Without this, googlevideo 403s the fetch.
#[tokio::test]
async fn full_get_sends_range_header_and_converts_206_to_200() {
    let app = axum::Router::new().route("/song", axum::routing::get(throttle_by_range));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
    let upstream_url = format!("http://{addr}/song");
    let (server, db, cache_dir) = test_start();
    let media = format!("{}/media/{}", server.base_url(), server.token);
    let res = reqwest::Client::new()
        .get(&media)
        .query(&[("url", upstream_url.as_str())])
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200, "206-from-zero must be served as 200");
    assert!(
        res.headers().get("content-range").is_none(),
        "full-body clients must not see Content-Range"
    );
    let body = res.bytes().await.unwrap();
    assert_eq!(body.len(), 2048);
    assert_eq!(body[0], 7);
    let files: Vec<_> = std::fs::read_dir(&cache_dir).unwrap().map(|e| e.unwrap()).collect();
    assert_eq!(files.len(), 1, "tee must cache the whole body");
    assert_eq!(files[0].metadata().unwrap().len(), 2048);
    let conn = db.lock().unwrap();
    assert!(cache::cache_index_get(&conn, &upstream_url).is_some());
}

#[tokio::test]
async fn cache_index_roundtrip_bookkeeping() {
    let conn = crate::db::open_memory().expect("db");
    let url = "https://example.com/song.m4a";

    assert!(cache::cache_index_get(&conn, url).is_none());
    cache::cache_index_put(&conn, url, "C:\\cache\\abc.m4a", Some("audio/mp4"), 4096).expect("put");

    let row = cache::cache_index_get(&conn, url).expect("row");
    assert_eq!(row.local_path, "C:\\cache\\abc.m4a");
    assert_eq!(row.bytes, 4096);

    cache::cache_index_touch(&conn, url).expect("touch 1");
    cache::cache_index_touch(&conn, url).expect("touch 2");
    let hits: i64 = conn
        .query_row(
            "SELECT hit_count FROM cache_index WHERE url = ?1",
            [url],
            |r| r.get(0),
        )
        .expect("hits");
    assert_eq!(hits, 2, "hit_count must increment per touch");

    // Upsert overwrites the same URL (re-download) and resets last_access.
    cache::cache_index_put(&conn, url, "C:\\cache\\xyz.webm", Some("audio/webm"), 8192)
        .expect("put2");
    let row = cache::cache_index_get(&conn, url).expect("row2");
    assert_eq!(row.local_path, "C:\\cache\\xyz.webm");
    assert_eq!(row.bytes, 8192);
}

#[tokio::test]
async fn cache_path_is_deterministic_and_mime_suffixed() {
    let dir = std::path::Path::new("C:\\cache");
    let url = "https://example.com/song";
    let a = cache::cache_path_for_url(dir, url, "audio/mp4");
    let b = cache::cache_path_for_url(dir, url, "audio/mp4");
    let c = cache::cache_path_for_url(dir, url, "audio/webm");
    assert_eq!(a, b, "same url+mime must map to same path");
    assert_ne!(a, c, "different mime must map to different path");
    assert_eq!(a.extension().unwrap(), "m4a");
    assert_eq!(c.extension().unwrap(), "webm");
}

/// Proxy Range against a real upstream: 206 forwarded with correct slice,
/// and partial responses are NOT written to the cache.
#[tokio::test]
async fn remote_range_get_forwards_206_and_skips_cache() {
    let (upstream, _udb, _udir) = test_start();
    let upstream_path = temp_file("upstream-range.mp3");
    let upstream_url = upstream.media_url(&upstream_path);

    let (server, db, cache_dir) = test_start();
    let media = format!("{}/media/{}", server.base_url(), server.token);

    let res = reqwest::Client::new()
        .get(&media)
        .query(&[("url", upstream_url.as_str())])
        .header("range", "bytes=1000-1999")
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 206);
    assert_eq!(
        res.headers().get("content-range").unwrap(),
        "bytes 1000-1999/204800"
    );
    let body = res.bytes().await.unwrap();
    assert_eq!(body.len(), 1000);
    assert_eq!(body[0], (1000 % 251) as u8);

    let cached = std::fs::read_dir(&cache_dir).unwrap().count();
    assert_eq!(cached, 0, "range responses must not be cached");
    let conn = db.lock().unwrap();
    assert!(cache::cache_index_get(&conn, &upstream_url).is_none());
}

/// Proxy full GET: body streams through, disk cache written via temp→rename,
/// cache_index row created; a second GET after upstream shutdown is served
/// from the disk cache (fast path).
#[tokio::test]
async fn remote_full_get_writes_cache_then_serves_offline() {
    let mut upstream = test_start().0;
    let upstream_path = temp_file("upstream-full.mp3");
    let upstream_url = upstream.media_url(&upstream_path);

    let (server, db, cache_dir) = test_start();
    let media = format!("{}/media/{}", server.base_url(), server.token);

    let res = reqwest::Client::new()
        .get(&media)
        .query(&[("url", upstream_url.as_str())])
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    let body = res.bytes().await.unwrap();
    assert_eq!(body.len(), 200 * 1024);
    assert_eq!(body[100 * 1024], (100 * 1024 % 251) as u8);

    let files: Vec<_> = std::fs::read_dir(&cache_dir)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();
    assert_eq!(files.len(), 1, "exactly one cached file expected");
    assert_eq!(files[0].metadata().unwrap().len(), 200 * 1024);
    {
        let conn = db.lock().unwrap();
        let row = cache::cache_index_get(&conn, &upstream_url).expect("index row");
        assert_eq!(row.bytes, (200 * 1024) as i64);
    }

    // Upstream goes away: second GET must still succeed from the disk cache.
    upstream.shutdown();
    std::thread::sleep(std::time::Duration::from_millis(100));

    let res2 = reqwest::Client::new()
        .get(&media)
        .query(&[("url", upstream_url.as_str())])
        .header("range", "bytes=0-1")
        .send()
        .await
        .expect("request2");
    assert_eq!(res2.status(), 206);
    assert_eq!(res2.bytes().await.unwrap().len(), 2);

    let conn = db.lock().unwrap();
    let hits: i64 = conn
        .query_row(
            "SELECT hit_count FROM cache_index WHERE url = ?1",
            [&upstream_url],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hits, 1, "fast-path serve must bump hit_count");
}
