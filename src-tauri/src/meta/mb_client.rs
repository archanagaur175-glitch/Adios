//! MusicBrainz search client (P5.1): meta_cache-first fetch through the
//! global 1-req/s queue. A cache hit returns WITHOUT acquiring the queue
//! or the transport (plan gate: "cache hit skips queue").
//! Docs: <https://musicbrainz.org/doc/MusicBrainz_API> (root `ws/2/`,
//! `fmt=json`, "never more than ONE call per second", meaningful UA).

use std::sync::Arc;

use serde::de::DeserializeOwned;

use super::mb_types::{self, SearchKind, SearchResults};
use super::rate_limit::RateLimitQueue;
use crate::resolver::{fnv1a_hex, BoxFuture, SharedDb};

/// D2: descriptive UA. Placeholder until the user saves their contact email
/// in settings (P5 adds the field); still "meaningful" for MB's rules.
pub fn user_agent(contact: Option<&str>) -> String {
    match contact.map(str::trim).filter(|s| !s.is_empty()) {
        Some(c) => format!("Adios/0.1 (personal use; contact: {c})"),
        None => "Adios/0.1 (personal use; contact: not-configured)".to_owned(),
    }
}

/// Normalized cache key: `mb_<kind>|<query>` → FNV-1a hex (same hash the
/// resolution cache uses). Case/whitespace-normalized so trivially
/// different queries share one row.
pub fn cache_key(kind: SearchKind, query: &str) -> String {
    let norm = query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    fnv1a_hex(&format!("{}|{}", kind.cache_kind(), norm))
}

/// `https://musicbrainz.org/ws/2/<entity>/?query=...&limit=25&fmt=json`
/// (percent-encoding via `reqwest::Url` query pairs).
pub fn build_url(kind: SearchKind, query: &str) -> Result<String, String> {
    let mut url =
        reqwest::Url::parse(&format!("https://musicbrainz.org/ws/2/{}/", kind.endpoint()))
            .map_err(|e| format!("mb: bad base url: {e}"))?;
    url.query_pairs_mut()
        .append_pair("query", query)
        .append_pair("limit", "25")
        .append_pair("fmt", "json");
    Ok(url.to_string())
}

/// HTTP GET behind a trait so tests can assert cache/queue ordering with a
/// recording mock instead of the network.
pub trait MbTransport: Send + Sync + 'static {
    fn get<'a>(
        &'a self,
        url: &'a str,
        user_agent: &'a str,
    ) -> BoxFuture<'a, Result<String, String>>;
}

pub struct ReqwestTransport {
    http: reqwest::Client,
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }
}

impl MbTransport for ReqwestTransport {
    fn get<'a>(&'a self, url: &'a str, ua: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let res = self
                .http
                .get(url)
                .header(reqwest::header::USER_AGENT, ua)
                .send()
                .await
                .map_err(|e| format!("mb: request failed: {e}"))?;
            let status = res.status();
            if !status.is_success() {
                return Err(format!("mb: HTTP {status}"));
            }
            res.text().await.map_err(|e| format!("mb: read failed: {e}"))
        })
    }
}

pub struct MbClient<T: MbTransport> {
    transport: T,
    queue: Arc<RateLimitQueue>,
    db: SharedDb,
    ua: String,
}

impl MbClient<ReqwestTransport> {
    /// Production wiring: reqwest transport + the shared 1-req/s queue.
    pub fn with_default_transport(db: SharedDb, ua: String) -> Self {
        Self::new(
            ReqwestTransport::default(),
            RateLimitQueue::one_per_second(),
            db,
            ua,
        )
    }
}

impl<T: MbTransport> MbClient<T> {
    pub fn new(transport: T, queue: Arc<RateLimitQueue>, db: SharedDb, ua: String) -> Self {
        Self {
            transport,
            queue,
            db,
            ua,
        }
    }

    /// Queue handle — tests assert cache hits never acquire a slot.
    #[cfg(test)]
    pub fn queue(&self) -> &Arc<RateLimitQueue> {
        &self.queue
    }

    fn cache_get(&self, kind: SearchKind, key: &str) -> Result<Option<String>, String> {
        let conn = self.db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT payload_json FROM meta_cache WHERE cache_key = ?1 AND kind = ?2")
            .map_err(|e| format!("mb: cache read: {e}"))?;
        let mut rows = stmt
            .query_map(rusqlite::params![key, kind.cache_kind()], |r| r.get(0))
            .map_err(|e| format!("mb: cache read: {e}"))?;
        rows.next()
            .transpose()
            .map_err(|e| format!("mb: cache read: {e}"))
    }

    fn cache_put(&self, kind: SearchKind, key: &str, body: &str) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        conn.execute(
            "INSERT INTO meta_cache (cache_key, kind, payload_json) VALUES (?1, ?2, ?3)
             ON CONFLICT(cache_key) DO UPDATE SET payload_json = excluded.payload_json,
               fetched_at = datetime('now')",
            rusqlite::params![key, kind.cache_kind(), body],
        )
        .map_err(|e| format!("mb: cache write: {e}"))?;
        Ok(())
    }

    /// Cache-first fetch. Hit → parsed body returned with zero queue slots
    /// and zero transport calls; miss → queue wait → GET → write-back.
    pub async fn fetch(&self, kind: SearchKind, query: &str) -> Result<String, String> {
        let key = cache_key(kind, query);
        if let Some(hit) = self.cache_get(kind, &key)? {
            println!("[mb] cache hit kind={} q={query}", kind.cache_kind());
            return Ok(hit);
        }
        self.queue.wait().await;
        println!("[mb] fetch kind={} q={query}", kind.cache_kind());
        let url = build_url(kind, query)?;
        let body = self.transport.get(&url, &self.ua).await?;
        if let Err(e) = self.cache_put(kind, &key, &body) {
            eprintln!("[mb] cache write skipped: {e}");
        }
        Ok(body)
    }

    /// Typed search: fetch (cache-first) + decode the documented list shape.
    pub async fn search<R: DeserializeOwned>(
        &self,
        kind: SearchKind,
        query: &str,
    ) -> Result<SearchResults<R>, String> {
        let body = self.fetch(kind, query).await?;
        mb_types::parse_list(kind.list_key(), &body)
    }
}
