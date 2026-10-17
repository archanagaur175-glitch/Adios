//! P16.5 (D): iTunes RSS top-songs chart client for Home's "Discover new"
//! — behind a **6 h age-aware `meta_cache`** (kind `itunes_rss`, using the
//! existing `fetched_at` column): a fresh row is served from the DB with
//! zero network, a stale one refetches through the shared 400 ms iTunes
//! spacing queue, and a failed refresh falls back to the stale payload so
//! an offline Home keeps its last chart. Types + parser live in
//! `itunes_rss_types.rs` (feed shape verified live 2026-10-17).

use std::sync::Arc;

use super::itunes::{ItTransport, ReqwestItTransport};
use super::rate_limit::RateLimitQueue;
use crate::resolver::SharedDb;

pub use super::itunes_rss_types::{parse_feed, RssSongOut};

pub const FEED_URL: &str = "https://itunes.apple.com/us/rss/topsongs/limit=25/json";
const KIND: &str = "itunes_rss";
const CACHE_KEY: &str = "rss|topsongs";
const MAX_AGE_HOURS: i64 = 6;

pub struct RssClient<T: ItTransport> {
    transport: T,
    queue: Arc<RateLimitQueue>,
    db: SharedDb,
}

impl RssClient<ReqwestItTransport> {
    pub fn with_default_transport(db: SharedDb) -> Self {
        Self::new(
            ReqwestItTransport::default(),
            RateLimitQueue::new(std::time::Duration::from_millis(400)),
            db,
        )
    }
}

impl<T: ItTransport> RssClient<T> {
    pub fn new(transport: T, queue: Arc<RateLimitQueue>, db: SharedDb) -> Self {
        Self { transport, queue, db }
    }

    #[cfg(test)]
    pub fn queue(&self) -> &Arc<RateLimitQueue> {
        &self.queue
    }

    /// `(payload_json, age_hours)` for the chart row; NULL age reads stale.
    fn cache_read(&self) -> Result<Option<(String, i64)>, String> {
        let conn = self.db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT payload_json,
                        CAST((julianday('now') - julianday(fetched_at)) * 24.0 AS INTEGER)
                   FROM meta_cache WHERE cache_key = ?1 AND kind = ?2",
            )
            .map_err(|e| format!("itunes_rss: cache read: {e}"))?;
        let mut rows = stmt
            .query_map(rusqlite::params![CACHE_KEY, KIND], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?.unwrap_or(i64::MAX)))
            })
            .map_err(|e| format!("itunes_rss: cache read: {e}"))?;
        rows.next().transpose().map_err(|e| format!("itunes_rss: cache read: {e}"))
    }

    fn cache_write(&self, body: &str) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        conn.execute(
            "INSERT INTO meta_cache (cache_key, kind, payload_json) VALUES (?1, ?2, ?3)
             ON CONFLICT(cache_key) DO UPDATE SET payload_json = excluded.payload_json,
               kind = excluded.kind, fetched_at = datetime('now')",
            rusqlite::params![CACHE_KEY, KIND, body],
        )
        .map_err(|e| format!("itunes_rss: cache write: {e}"))?;
        Ok(())
    }

    /// Age-aware chart: fresh (<6 h) → cache only; stale → space → GET →
    /// write-back; refresh failure with any prior row → serve it stale.
    pub async fn top_songs(&self) -> Result<Vec<RssSongOut>, String> {
        let cached = self.cache_read()?;
        if let Some((payload, age)) = &cached {
            if *age < MAX_AGE_HOURS {
                println!("[itunes_rss] cache hit age={age}h");
                return parse_feed(payload);
            }
        }
        self.queue.wait().await;
        println!("[itunes_rss] fetch {FEED_URL}");
        match self.transport.get(FEED_URL).await {
            Ok(body) => {
                if let Err(e) = self.cache_write(&body) {
                    eprintln!("[itunes_rss] cache write skipped: {e}");
                }
                parse_feed(&body)
            }
            Err(e) => {
                if let Some((payload, _)) = cached {
                    eprintln!("[itunes_rss] refresh failed ({e}); serving stale cache");
                    return parse_feed(&payload);
                }
                Err(e)
            }
        }
    }
}
