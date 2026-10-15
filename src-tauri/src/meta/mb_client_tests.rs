//! P5.1 tests: cache/queue/transport ordering + URL, key, UA construction.
//! The mock transport proves "cache hit skips queue" WITHOUT the network.

use std::sync::{Arc, Mutex};

use crate::db::open_memory;
use crate::meta::mb_client::{build_url, cache_key, user_agent, MbClient, MbTransport};
use crate::meta::mb_types::{MbArtist, MbRecording, SearchKind};
use crate::meta::rate_limit::RateLimitQueue;
use crate::resolver::{BoxFuture, SharedDb};

const RECORDING_JSON: &str = r#"{
  "count": 1, "offset": 0,
  "recordings": [
    { "id": "026fa041-3917-4c73-9079-ed16e36f20f8", "score": "100",
      "title": "Blow Your Mind (Mwah)", "length": 178000,
      "artist-credit": [ { "artist": { "name": "Dua Lipa" } } ] }
  ]
}"#;

/// Call log shared with the test — MbClient's transport field stays private.
#[derive(Default)]
struct MockTransport {
    calls: Arc<Mutex<Vec<(String, String)>>>,
    body: String,
}

impl MbTransport for MockTransport {
    fn get<'a>(&'a self, url: &'a str, ua: &'a str) -> BoxFuture<'a, Result<String, String>> {
        self.calls
            .lock()
            .unwrap()
            .push((url.to_owned(), ua.to_owned()));
        let body = self.body.clone();
        Box::pin(async move { Ok(body) })
    }
}

type Harness = (
    MbClient<MockTransport>,
    Arc<RateLimitQueue>,
    SharedDb,
    Arc<Mutex<Vec<(String, String)>>>,
);

fn client(body: &str, interval: std::time::Duration) -> Harness {
    let db: SharedDb = Arc::new(Mutex::new(open_memory().unwrap()));
    let queue = RateLimitQueue::new(interval);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let transport = MockTransport {
        calls: Arc::clone(&calls),
        body: body.to_owned(),
    };
    let client = MbClient::new(
        transport,
        Arc::clone(&queue),
        Arc::clone(&db),
        user_agent(None),
    );
    (client, queue, db, calls)
}

#[tokio::test]
async fn cache_hit_skips_queue_and_transport() {
    let (c, queue, db, calls) = client(RECORDING_JSON, std::time::Duration::from_secs(1));
    let query = "  Dua Lipa - Blow Your Mind ";
    {
        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO meta_cache (cache_key, kind, payload_json) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                cache_key(SearchKind::Recording, query),
                SearchKind::Recording.cache_kind(),
                RECORDING_JSON
            ],
        )
        .unwrap();
    }

    // Different case/spacing must still normalize onto the seeded key.
    let page = c
        .search::<MbRecording>(SearchKind::Recording, "dua lipa - blow your mind")
        .await
        .expect("cached decode");

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].title, "Blow Your Mind (Mwah)");
    assert_eq!(queue.acquired(), 0, "cache hit must not take a rate slot");
    assert!(calls.lock().unwrap().is_empty(), "cache hit must not call transport");
}

#[tokio::test]
async fn miss_fetches_writes_back_then_repeats_are_cache_only() {
    let (c, queue, db, calls) = client(RECORDING_JSON, std::time::Duration::from_millis(1));
    let query = "Dua Lipa - Blow Your Mind";

    let first = c
        .search::<MbRecording>(SearchKind::Recording, query)
        .await
        .expect("first fetch");
    assert_eq!(first.items.len(), 1);
    assert_eq!(queue.acquired(), 1);
    assert_eq!(calls.lock().unwrap().len(), 1);
    {
        let (url, ua) = &calls.lock().unwrap()[0];
        assert!(
            url.starts_with("https://musicbrainz.org/ws/2/recording/"),
            "url: {url}"
        );
        assert!(url.contains("fmt=json") && url.contains("limit=25"), "url: {url}");
        assert!(ua.starts_with("Adios/0.1"), "ua: {ua}");
    }
    {
        let conn = db.lock().unwrap();
        let rows: i64 = conn
            .query_row(
                "SELECT count(*) FROM meta_cache WHERE kind = 'mb_recording'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 1, "write-back must land in meta_cache");
    }

    let again = c
        .search::<MbRecording>(SearchKind::Recording, query)
        .await
        .expect("second fetch");
    assert_eq!(again.items[0].id, first.items[0].id);
    assert_eq!(queue.acquired(), 1, "repeat must not re-acquire a slot");
    assert_eq!(calls.lock().unwrap().len(), 1, "repeat must not re-hit the transport");
}

#[tokio::test]
async fn different_kinds_get_different_cache_rows() {
    let (c, queue, _db, calls) = client(RECORDING_JSON, std::time::Duration::from_millis(1));
    let _ = c.search::<MbRecording>(SearchKind::Recording, "same query").await;
    let _ = c.search::<MbArtist>(SearchKind::Artist, "same query").await;
    assert_eq!(calls.lock().unwrap().len(), 2, "kind is part of cache identity");
    assert_eq!(queue.acquired(), 2);
    assert_ne!(
        cache_key(SearchKind::Recording, "same query"),
        cache_key(SearchKind::Artist, "same query")
    );
}

#[test]
fn url_matches_documented_shape_and_encodes_quotes() {
    let url = build_url(SearchKind::Recording, "We \"Will Rock You\"").unwrap();
    assert!(
        url.starts_with("https://musicbrainz.org/ws/2/recording/?"),
        "url: {url}"
    );
    assert!(url.contains("fmt=json"), "url: {url}");
    assert!(url.contains("limit=25"), "url: {url}");
    assert!(url.contains("%22"), "quotes must be percent-encoded: {url}");
}

#[test]
fn cache_key_normalizes_query_whitespace_and_case() {
    assert_eq!(
        cache_key(SearchKind::Artist, "  The   WEEKND "),
        cache_key(SearchKind::Artist, "the weeknd")
    );
}

#[test]
fn user_agent_is_descriptive_with_or_without_contact() {
    assert_eq!(
        user_agent(Some("  me@example.io ")),
        "Adios/0.1 (personal use; contact: me@example.io)"
    );
    assert!(user_agent(None).contains("not-configured"));
    assert!(user_agent(Some("  ")).contains("not-configured"));
}
