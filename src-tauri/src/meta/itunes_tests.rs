//! P9.7 tests: iTunes client cache/queue ordering + parse + URL.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::itunes::{
    build_url, cache_key, parse_and_rank, ItTransport, ItunesClient,
};
use crate::resolver::BoxFuture;

const PAGE: &str = r#"{"resultCount":3,"results":[
  {"trackName":"Never Gonna Give You Up","artistName":"Rick Astley",
   "collectionName":"Whenever You Need Somebody","trackTimeMillis":213573,
   "trackId":1438132740,
   "artworkUrl100":"https://is1-ssl.mzstatic.com/image/thumb/Music125/v4/art/1438132740/100x100bb.jpg",
   "primaryGenreName":"Pop","releaseDate":"2016-07-01T12:00:00-07:00"},
  {"trackName":"Never Gonna Give You Up (2022 Remaster)","artistName":"Rick Astley",
   "collectionName":"Whenever You Need Somebody (2022 Remaster)",
   "trackTimeMillis":213560,"trackId":1700000001},
  {"trackName":"Husn","artistName":"Anuv Jain","trackTimeMillis":180000,
   "trackId":1700000002}
]}"#;

struct MockTransport {
    body: String,
    calls: AtomicUsize,
}

impl ItTransport for MockTransport {
    fn get<'a>(&'a self, _url: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.body.clone())
        })
    }
}

fn client(
    body: &str,
    interval_ms: u64,
) -> (ItunesClient<MockTransport>, Arc<super::rate_limit::RateLimitQueue>) {
    let conn = crate::db::open_memory().expect("db");
    let db = crate::db::DbState::new(conn).shared();
    let transport = MockTransport { body: body.to_owned(), calls: AtomicUsize::new(0) };
    let queue =
        super::rate_limit::RateLimitQueue::new(std::time::Duration::from_millis(interval_ms));
    (ItunesClient::new(transport, Arc::clone(&queue), db), queue)
}

#[test]
fn build_url_uses_term_param_and_entity_song() {
    let url = build_url("daft punk").expect("url");
    assert!(url.starts_with("https://itunes.apple.com/search?"));
    assert!(url.contains("term=daft+punk") || url.contains("term=daft%20punk"));
    assert!(url.contains("media=music"));
    assert!(url.contains("entity=song"));
    assert!(url.contains("limit=25"));
}

#[test]
fn parse_keeps_relevance_order_and_maps_fields() {
    let rows = parse_and_rank(PAGE).expect("parse");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].itunes_id, 1438132740);
    assert_eq!(rows[0].title, "Never Gonna Give You Up");
    assert_eq!(rows[0].artist, "Rick Astley");
    assert_eq!(rows[0].album.as_deref(), Some("Whenever You Need Somebody"));
    assert_eq!(rows[0].duration_ms, Some(213573));
    assert_eq!(
        rows[0].artwork_url.as_deref(),
        Some("https://is1-ssl.mzstatic.com/image/thumb/Music125/v4/art/1438132740/100x100bb.jpg"),
        "P12: artworkUrl100 plumbed through"
    );
    assert_eq!(rows[1].itunes_id, 1700000001, "relevance order preserved");
    assert_eq!(rows[1].artwork_url, None, "missing artwork stays None");
    assert_eq!(rows[0].genre.as_deref(), Some("Pop"), "P16: primaryGenreName");
    assert_eq!(
        rows[0].release_date.as_deref(),
        Some("2016-07-01T12:00:00-07:00"),
        "P16: releaseDate carried"
    );
    assert_eq!(rows[2].genre, None, "missing genre stays None");
}

#[test]
fn parse_rejects_garbage_and_accepts_empty_results() {
    assert!(parse_and_rank("not json").is_err());
    assert!(parse_and_rank(r#"{"resultCount":0,"results":[]}"#).is_ok());
}

#[tokio::test]
async fn cache_hit_skips_queue_and_transport() {
    let (c, _queue) = client(PAGE, 1);
    let first = c.search("rick astley").await.expect("first");
    assert_eq!(first.len(), 3);
    let second = c.search("rick astley").await.expect("second");
    assert_eq!(second.len(), 3);
    assert_eq!(c.queue().acquired(), 1, "repeat must be cache-only");
}

#[tokio::test]
async fn two_distinct_queries_take_two_slots() {
    let (c, _queue) = client(PAGE, 1);
    c.search("a").await.expect("a");
    c.search("b").await.expect("b");
    assert_eq!(c.queue().acquired(), 2);
}

#[test]
fn cache_key_normalizes_case_and_whitespace() {
    assert_eq!(cache_key("Daft  Punk"), cache_key("daft punk"));
    assert_ne!(cache_key("daft punk"), cache_key("daft punk live"));
}
