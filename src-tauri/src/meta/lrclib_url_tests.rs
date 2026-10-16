//! P7.1 tests part 2: fallback / transient-recovery behavior + pure URL and
//! cache-key construction. Shares the mock harness from `lrclib_tests`.

use super::lrclib_tests::{client, ok200, query, status, INSTRUMENTAL_BODY, RECORD_BODY};
use crate::meta::lrclib::{build_url, cache_key};
use crate::meta::lrclib_types::LyricsQuery;

#[tokio::test]
async fn durationless_fallback_fires_after_duration_filtered_404() {
    let (c, _queue, _db, calls) = client(vec![status(404), ok200(RECORD_BODY)]);

    let hit = c
        .fetch(&query())
        .await
        .expect("fallback fetch")
        .expect("hit");
    assert_eq!(hit.lrclib_id, Some(3712470));
    let urls = calls.lock().unwrap().clone();
    assert_eq!(urls.len(), 2);
    assert!(urls[0].contains("duration=261"), "strict url: {}", urls[0]);
    assert!(!urls[1].contains("duration="), "fallback url: {}", urls[1]);
}

#[tokio::test]
async fn instrumental_200_is_negative_not_hit() {
    let (c, _queue, db, calls) = client(vec![ok200(INSTRUMENTAL_BODY)]);
    // Instrumental records may carry a different duration — build a query
    // whose duration matches the canned body so the strict pass hits it.
    let q = LyricsQuery {
        artist: "Noone".into(),
        title: "Vordrex".into(),
        album: None,
        duration_ms: Some(100_000),
    };

    let first = c.fetch(&q).await.expect("first fetch");
    assert!(first.is_none(), "200 instrumental must map to Ok(None)");
    assert_eq!(calls.lock().unwrap().len(), 1);
    {
        let conn = db.lock().unwrap();
        let (row_status, instrumental): (String, i64) = conn
            .query_row(
                "SELECT status, instrumental FROM lyrics_cache WHERE track_key = ?1",
                rusqlite::params![cache_key(&q)],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(row_status, "not_found");
        assert_eq!(instrumental, 1, "reason preserved on the negative row");
    }

    let again = c.fetch(&q).await.expect("repeat fetch");
    assert!(again.is_none());
    assert_eq!(calls.lock().unwrap().len(), 1, "negative row blocks refetch");
}

#[tokio::test(start_paused = true)]
async fn overloaded_503_retries_once_then_errors_and_never_caches() {
    let (c, _queue, db, calls) = client(vec![status(503), status(503)]);

    let err = c.fetch(&query()).await.expect_err("persistent 503 errors");
    assert!(err.contains("503"), "err: {err}");
    assert_eq!(calls.lock().unwrap().len(), 2, "one retry after the first 503");
    {
        let conn = db.lock().unwrap();
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM lyrics_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0, "transient overload must never write a row");
    }
}

#[tokio::test(start_paused = true)]
async fn single_503_then_success_recovers_via_retry() {
    let (c, _queue, _db, calls) = client(vec![status(503), ok200(RECORD_BODY)]);

    let hit = c.fetch(&query()).await.expect("retry fetch").expect("hit");
    assert_eq!(hit.lrclib_id, Some(3712470));
    // Clone under ONE guard: asserting over two live `calls.lock()`
    // temporaries in the same expression would self-deadlock the std Mutex.
    let urls = calls.lock().unwrap().clone();
    assert_eq!(urls.len(), 2, "retry re-hits the same URL");
    assert_eq!(urls[0], urls[1]);
}

#[test]
fn url_matches_live_shape_and_encodes_spaces() {
    let url = build_url(&query(), true).unwrap();
    assert!(url.starts_with("https://lrclib.net/api/get?"), "url: {url}");
    assert!(url.contains("artist_name=Radiohead"), "url: {url}");
    assert!(url.contains("track_name=Karma+Police"), "url: {url}");
    assert!(url.contains("album_name=OK+Computer"), "url: {url}");
    assert!(url.contains("duration=261"), "url: {url}");

    let bare = build_url(&query(), false).unwrap();
    assert!(!bare.contains("duration="), "url: {bare}");
}

#[test]
fn cache_key_normalizes_case_and_whitespace() {
    let a = LyricsQuery {
        artist: "  The   WEEKND ".into(),
        title: " Blinding  Lights ".into(),
        album: None,
        duration_ms: Some(200_000),
    };
    let b = LyricsQuery {
        artist: "the weeknd".into(),
        title: "blinding lights".into(),
        album: None,
        duration_ms: Some(200_000),
    };
    assert_eq!(cache_key(&a), cache_key(&b));
    let c = LyricsQuery {
        duration_ms: Some(201_000),
        ..b.clone()
    };
    assert_ne!(cache_key(&b), cache_key(&c), "duration is part of the key");
}
