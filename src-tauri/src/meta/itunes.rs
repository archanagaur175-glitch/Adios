//! iTunes search client (P9.7): the keyless track picker for the Ctrl+K
//! overlay. (Planned as Deezer, but their anonymous API now returns empty
//! `data` arrays — swapped to iTunes Search API, which needs no key and
//! returns title/artist/album/duration in relevance order.) Single request,
//! cached through `meta_cache` (kind "itunes"), same pattern as MB/Deezer
//! clients. Docs: <https://performance-partners.apple.com/search-api>
//! (anonymous, no documented rate limit; we space at 400 ms anyway).

use std::sync::Arc;

use serde::Deserialize;

use super::rate_limit::RateLimitQueue;
use crate::resolver::{fnv1a_hex, BoxFuture, SharedDb};

/// Anonymous politeness spacing.
const REQUEST_INTERVAL_MS: u64 = 400;

#[derive(Debug, Clone, Deserialize)]
struct ItEntry {
    #[serde(rename = "trackName")]
    title: String,
    #[serde(rename = "artistName")]
    artist: String,
    #[serde(rename = "collectionName")]
    album: Option<String>,
    #[serde(rename = "trackTimeMillis")]
    duration_ms: Option<i64>,
    #[serde(rename = "trackId")]
    track_id: i64,
    /// 100×100 px artwork URL (P12 covers). The 60px variant is derivable
    /// by swapping `100x100` → `60x60` when a smaller thumb is wanted.
    #[serde(rename = "artworkUrl100")]
    artwork_url: Option<String>,
    /// P16 taste shelves: Apple's broad genre bucket (e.g. "Electronic").
    #[serde(rename = "primaryGenreName")]
    genre: Option<String>,
    /// P16 discover: ISO-8601 release stamp, carried for freshness checks.
    #[serde(rename = "releaseDate")]
    release_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ItPage {
    #[serde(default)]
    results: Vec<ItEntry>,
}

/// One picker row for the webview (camelCase, matches OnlineResult).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItunesTrackOut {
    pub itunes_id: i64,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
    pub artwork_url: Option<String>,
    pub genre: Option<String>,
    pub release_date: Option<String>,
}

/// Normalized cache key: `itunes|<query>` → FNV-1a hex (MB-style).
pub fn cache_key(query: &str) -> String {
    let norm = query.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    fnv1a_hex(&format!("itunes|{norm}"))
}

/// `https://itunes.apple.com/search?term=...&media=music&entity=song&limit=25`
/// (percent-encoded; `term` is the iTunes param name).
pub fn build_url(query: &str) -> Result<String, String> {
    let mut url = reqwest::Url::parse("https://itunes.apple.com/search")
        .map_err(|e| format!("itunes: bad base url: {e}"))?;
    url.query_pairs_mut()
        .append_pair("term", query)
        .append_pair("media", "music")
        .append_pair("entity", "song")
        .append_pair("limit", "25");
    Ok(url.to_string())
}

/// HTTP GET behind a trait so tests can mock the network.
pub trait ItTransport: Send + Sync + 'static {
    fn get<'a>(&'a self, url: &'a str) -> BoxFuture<'a, Result<String, String>>;
}

pub struct ReqwestItTransport {
    http: reqwest::Client,
}

impl Default for ReqwestItTransport {
    fn default() -> Self {
        Self { http: crate::net::http_client() }
    }
}

impl ItTransport for ReqwestItTransport {
    fn get<'a>(&'a self, url: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let res = self
                .http
                .get(url)
                .send()
                .await
                .map_err(|e| format!("itunes: request failed: {e}"))?;
            let status = res.status();
            if !status.is_success() {
                return Err(format!("itunes: HTTP {status}"));
            }
            res.text().await.map_err(|e| format!("itunes: read failed: {e}"))
        })
    }
}

pub struct ItunesClient<T: ItTransport> {
    transport: T,
    queue: Arc<RateLimitQueue>,
    db: SharedDb,
}

impl ItunesClient<ReqwestItTransport> {
    pub fn with_default_transport(db: SharedDb) -> Self {
        Self::new(
            ReqwestItTransport::default(),
            RateLimitQueue::new(std::time::Duration::from_millis(REQUEST_INTERVAL_MS)),
            db,
        )
    }
}

/// Decode a raw iTunes search body (pure, unit-tested). Relevance order
/// from iTunes is preserved (no explicit rank field — first = best match).
pub fn parse_and_rank(body: &str) -> Result<Vec<ItunesTrackOut>, String> {
    let page: ItPage = serde_json::from_str(body).map_err(|e| format!("itunes: bad json: {e}"))?;
    Ok(page
        .results
        .into_iter()
        .map(|t| ItunesTrackOut {
            itunes_id: t.track_id,
            title: t.title,
            artist: t.artist,
            album: t.album,
            duration_ms: t.duration_ms,
            artwork_url: t.artwork_url,
            genre: t.genre,
            release_date: t.release_date,
        })
        .collect())
}

impl<T: ItTransport> ItunesClient<T> {
    pub fn new(transport: T, queue: Arc<RateLimitQueue>, db: SharedDb) -> Self {
        Self { transport, queue, db }
    }

    #[cfg(test)]
    pub fn queue(&self) -> &Arc<RateLimitQueue> {
        &self.queue
    }

    fn cache_get(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT payload_json FROM meta_cache WHERE cache_key = ?1 AND kind = 'itunes'")
            .map_err(|e| format!("itunes: cache read: {e}"))?;
        let mut rows = stmt.query_map([key], |r| r.get::<_, String>(0)).map_err(|e| format!("itunes: cache read: {e}"))?;
        rows.next().transpose().map_err(|e| format!("itunes: cache read: {e}"))
    }

    fn cache_put(&self, key: &str, body: &str) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        conn.execute(
            "INSERT INTO meta_cache (cache_key, kind, payload_json) VALUES (?1, 'itunes', ?2)
             ON CONFLICT(cache_key) DO UPDATE SET payload_json = excluded.payload_json,
               fetched_at = datetime('now')",
            rusqlite::params![key, body],
        )
        .map_err(|e| format!("itunes: cache write: {e}"))?;
        Ok(())
    }

    /// Cache-first search: hit → zero queue/transport; miss → space → GET →
    /// write-back → parse (relevance order kept).
    pub async fn search(&self, query: &str) -> Result<Vec<ItunesTrackOut>, String> {
        let key = cache_key(query);
        if let Some(hit) = self.cache_get(&key)? {
            println!("[itunes] cache hit q={query}");
            return parse_and_rank(&hit);
        }
        self.queue.wait().await;
        println!("[itunes] fetch q={query}");
        let url = build_url(query)?;
        let body = self.transport.get(&url).await?;
        if let Err(e) = self.cache_put(&key, &body) {
            eprintln!("[itunes] cache write skipped: {e}");
        }
        parse_and_rank(&body)
    }
}
