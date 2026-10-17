//! P16.5 tests: RSS feed parse (name preference, suffix strip, tallest
//! image) + age-aware cache (fresh hit / stale refetch / stale-on-error).

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use super::itunes_rss::{parse_feed, RssClient, FEED_URL};
use super::rate_limit::RateLimitQueue;
use crate::resolver::{BoxFuture, SharedDb};

const FEED: &str = r#"{"feed":{"entry":[
  {"im:name":{"label":"expectations"},
   "im:image":[
     {"label":"https://cdn/img/55x55.png","attributes":{"height":"55"}},
     {"label":"https://cdn/img/170x170.png","attributes":{"height":"170"}},
     {"label":"https://cdn/img/60x60.png","attributes":{"height":"60"}}],
   "title":{"label":"expectations - Olivia Rodrigo & Chappell Roan"},
   "id":{"label":"https://music.apple.com/…","attributes":{"im:id":"6822390426"}},
   "im:artist":{"label":"Olivia Rodrigo & Chappell Roan"},
   "category":{"attributes":{"im:id":"14","term":"Pop"}},
   "im:releaseDate":{"label":"2026-10-16T05:00:00-07:00"}},
  {"title":{"label":"Husn - Anuv Jain"},
   "id":{"label":"https://music.apple.com/…","attributes":{"im:id":"1700000002"}},
   "im:artist":{"label":"Anuv Jain"}},
  {"title":{"label":"No Id Song"},
   "id":{"label":"https://music.apple.com/…"},
   "im:artist":{"label":"Ghost"}}
]}}"#;

struct MockTransport {
    body: String,
    fail: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}

impl super::itunes::ItTransport for MockTransport {
    fn get<'a>(&'a self, _url: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail.load(Ordering::SeqCst) {
                Err("rss: simulated network down".to_owned())
            } else {
                Ok(self.body.clone())
            }
        })
    }
}

type TestClient = (RssClient<MockTransport>, Arc<AtomicBool>, SharedDb);

fn client(body: &str) -> TestClient {
    let conn = crate::db::open_memory().expect("db");
    let db = crate::db::DbState::new(conn).shared();
    let fail = Arc::new(AtomicBool::new(false));
    let transport = MockTransport {
        body: body.to_owned(),
        fail: Arc::clone(&fail),
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let queue = RateLimitQueue::new(std::time::Duration::from_millis(1));
    (RssClient::new(transport, queue, Arc::clone(&db)), fail, db)
}

fn age_cache_to(db: &SharedDb, old: &str) {
    let conn = db.lock().expect("db");
    conn.execute("UPDATE meta_cache SET fetched_at = ?1", [old]).expect("age");
}

#[test]
fn parse_prefers_im_name_and_picks_tallest_image() {
    let rows = parse_feed(FEED).expect("parse");
    assert_eq!(rows.len(), 2, "im:id-less entry is skipped");
    assert_eq!(rows[0].itunes_id, 6822390426);
    assert_eq!(rows[0].title, "expectations", "im:name wins");
    assert_eq!(rows[0].artist, "Olivia Rodrigo & Chappell Roan");
    assert_eq!(rows[0].artwork_url.as_deref(), Some("https://cdn/img/170x170.png"));
    assert_eq!(rows[0].genre.as_deref(), Some("Pop"));
    assert_eq!(rows[0].release_date.as_deref(), Some("2026-10-16T05:00:00-07:00"));
}

#[test]
fn parse_falls_back_to_title_suffix_strip() {
    let rows = parse_feed(FEED).expect("parse");
    assert_eq!(rows[1].title, "Husn", "title suffix stripped when im:name absent");
    assert_eq!(rows[1].genre, None, "missing category stays None");
    assert_eq!(rows[1].artwork_url, None, "missing images stay None");
}

#[test]
fn parse_rejects_garbage_and_accepts_empty_feed() {
    assert!(parse_feed("not json").is_err());
    assert!(parse_feed(r#"{"feed":{}}"#).is_ok());
    assert!(parse_feed(r#"{"feed":{"entry":[]}}"#).unwrap().is_empty());
}

#[test]
fn feed_url_targets_top_25_json() {
    assert!(FEED_URL.ends_with("/topsongs/limit=25/json"));
}

#[tokio::test]
async fn fresh_cache_serves_without_refetch() {
    let (c, _fail, _db) = client(FEED);
    let first = c.top_songs().await.expect("first");
    assert_eq!(first.len(), 2);
    let second = c.top_songs().await.expect("second");
    assert_eq!(second.len(), 2);
    assert_eq!(c.queue().acquired(), 1, "repeat must be cache-only");
}

#[tokio::test]
async fn stale_cache_refetches_and_rewrites() {
    let (c, _fail, db) = client(FEED);
    c.top_songs().await.expect("first");
    age_cache_to(&db, "2026-01-01 00:00:00");
    let again = c.top_songs().await.expect("stale");
    assert_eq!(again.len(), 2);
    assert_eq!(c.queue().acquired(), 2, "stale must take a rate slot");
    // Write-back aged fetched_at back to now → next call is a fresh hit.
    let third = c.top_songs().await.expect("fresh");
    assert_eq!(third.len(), 2);
    assert_eq!(c.queue().acquired(), 2, "rewritten row is fresh again");
}

#[tokio::test]
async fn refresh_failure_serves_stale_payload() {
    let (c, fail, db) = client(FEED);
    c.top_songs().await.expect("first");
    age_cache_to(&db, "2026-01-01 00:00:00");
    fail.store(true, Ordering::SeqCst);
    let rows = c.top_songs().await.expect("stale fallback");
    assert_eq!(rows.len(), 2, "chart still served while offline");
    assert_eq!(c.queue().acquired(), 2, "attempted the refresh");
}

#[tokio::test]
async fn cold_fetch_failure_is_an_error() {
    let (c, fail, _db) = client(FEED);
    fail.store(true, Ordering::SeqCst);
    let err = c.top_songs().await.expect_err("no cache to fall back to");
    assert!(err.contains("simulated network down"), "got: {err}");
}
