//! P7.1 tests: cache-first / negative / transient semantics with a mock
//! transport — no network. Plan-required: negative row blocks the second
//! transport call; hit returns lyrics with zero transport.
//! Shared harness (pub for `lrclib_url_tests`).

use std::sync::{Arc, Mutex};

use crate::db::{lyrics_cache, open_memory};
use crate::meta::lrclib::{cache_key, LyricsClient, LrclibTransport, TransportResponse};
use crate::meta::lrclib_types::{LyricsPayload, LyricsQuery};
use crate::meta::rate_limit::RateLimitQueue;
use crate::resolver::SharedDb;

pub(super) const RECORD_BODY: &str = r#"{
  "id": 3712470, "name": "Karma Police", "trackName": "Karma Police",
  "artistName": "Radiohead", "albumName": "OK Computer", "duration": 261.0,
  "instrumental": false, "hasWordSync": true,
  "plainLyrics": "Karma police\nArrest this man",
  "syncedLyrics": "[00:26.83] Karma police\n[00:30.96] Arrest this man"
}"#;

pub(super) const INSTRUMENTAL_BODY: &str = r#"{
  "id": 9, "trackName": "Vordrex", "artistName": "Noone", "albumName": null,
  "duration": 100.0, "instrumental": true,
  "plainLyrics": null, "syncedLyrics": null
}"#;

pub(super) fn query() -> LyricsQuery {
    LyricsQuery {
        artist: "Radiohead".into(),
        title: "Karma Police".into(),
        album: Some("OK Computer".into()),
        duration_ms: Some(261_000),
    }
}

/// Queueing mock: every get() shifts the next canned reply.
/// (pub(super): the shared `Harness` alias names it.)
pub(super) struct MockTransport {
    calls: Arc<Mutex<Vec<String>>>,
    replies: Mutex<Vec<Result<TransportResponse, String>>>,
}

impl LrclibTransport for MockTransport {
    fn get<'a>(
        &'a self,
        url: &'a str,
    ) -> crate::resolver::BoxFuture<'a, Result<TransportResponse, String>> {
        self.calls.lock().unwrap().push(url.to_owned());
        // Vec::remove already yields the Result — hand it straight to the
        // block (no `.expect`, or the async block would resolve to the bare
        // TransportResponse and fail the trait's Result return type).
        let reply = self.replies.lock().unwrap().remove(0);
        Box::pin(async move { reply })
    }
}

pub(super) type Harness = (
    LyricsClient<MockTransport>,
    Arc<RateLimitQueue>,
    SharedDb,
    Arc<Mutex<Vec<String>>>,
);

pub(super) fn client(replies: Vec<Result<TransportResponse, String>>) -> Harness {
    let db: SharedDb = Arc::new(Mutex::new(open_memory().unwrap()));
    let queue = RateLimitQueue::new(std::time::Duration::from_millis(1));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let transport = MockTransport {
        calls: Arc::clone(&calls),
        replies: Mutex::new(replies),
    };
    let client = LyricsClient::new(transport, Arc::clone(&queue), Arc::clone(&db));
    (client, queue, db, calls)
}

pub(super) fn ok200(body: &str) -> Result<TransportResponse, String> {
    Ok(TransportResponse {
        status: 200,
        body: body.to_owned(),
    })
}

pub(super) fn status(code: u16) -> Result<TransportResponse, String> {
    Ok(TransportResponse {
        status: code,
        body: String::new(),
    })
}

#[tokio::test]
async fn cache_hit_skips_queue_and_transport() {
    let (c, queue, db, calls) = client(vec![]);
    let key = cache_key(&query());
    let seeded = LyricsPayload::from_record(serde_json::from_str(RECORD_BODY).unwrap(), "network");
    lyrics_cache::put_hit(&db, &key, &seeded).unwrap();

    let hit = c.fetch(&query()).await.expect("cached fetch");
    let hit = hit.expect("hit");
    assert!(hit.found);
    assert_eq!(hit.source, "cache");
    assert_eq!(hit.lrclib_id, Some(3712470));
    assert!(hit.synced.as_deref().unwrap().starts_with("[00:26.83]"));
    assert_eq!(queue.acquired(), 0, "cache hit must not take a rate slot");
    assert!(
        calls.lock().unwrap().is_empty(),
        "cache hit must not call transport"
    );
}

#[tokio::test]
async fn negative_cache_blocks_second_network_call() {
    // Query carries a duration → a 404 on the strict pass triggers the
    // durationless fallback (see lrclib.rs), so both calls 404 here.
    let (c, queue, db, calls) = client(vec![status(404), status(404)]);

    let first = c.fetch(&query()).await.expect("first fetch");
    assert!(first.is_none(), "404 maps to Ok(None)");
    assert_eq!(calls.lock().unwrap().len(), 2, "strict + durationless fallback");
    assert_eq!(queue.acquired(), 2);

    let again = c.fetch(&query()).await.expect("repeat fetch");
    assert!(again.is_none());
    assert_eq!(
        calls.lock().unwrap().len(),
        2,
        "negative row must block any further transport call"
    );
    assert_eq!(
        queue.acquired(),
        2,
        "negative cache hit must not re-acquire a slot"
    );
    {
        let conn = db.lock().unwrap();
        let (row_status, instrumental): (String, i64) = conn
            .query_row(
                "SELECT status, instrumental FROM lyrics_cache WHERE track_key = ?1",
                rusqlite::params![cache_key(&query())],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(row_status, "not_found");
        assert_eq!(instrumental, 0);
    }
}

#[tokio::test]
async fn miss_fetches_writes_hit_then_repeats_are_cache_only() {
    let (c, queue, db, calls) = client(vec![ok200(RECORD_BODY)]);

    let first = c.fetch(&query()).await.expect("first fetch").expect("hit");
    assert_eq!(first.source, "network");
    assert_eq!(first.plain.as_deref(), Some("Karma police\nArrest this man"));
    assert_eq!(calls.lock().unwrap().len(), 1);
    assert_eq!(queue.acquired(), 1);
    {
        let conn = db.lock().unwrap();
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM lyrics_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 1, "write-back must land in lyrics_cache");
    }

    let again = c.fetch(&query()).await.expect("repeat fetch").expect("hit");
    assert_eq!(again.source, "cache");
    assert_eq!(again.lrclib_id, first.lrclib_id);
    assert_eq!(queue.acquired(), 1, "repeat must not re-acquire a slot");
    assert_eq!(
        calls.lock().unwrap().len(),
        1,
        "repeat must not re-hit transport"
    );
}
