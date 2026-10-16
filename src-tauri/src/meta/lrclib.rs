//! LRCLIB lyrics client (P7.1): cache-first fetch through a dedicated
//! 1-req/s queue (LRCLIB has no documented app-level limit — server source
//! router.rs carries only metrics — but live probes 2026-10-16 hit Cloudflare
//! 503 + Retry-After: 1 on bursts, so we pace conservatively and share
//! NOTHING with the MB queue). Identification: `Lrclib-Client` header.
//! Cache semantics (plan gate): hit → zero queue/transport; 404 and
//! empty/instrumental 200 → negative row; 503 → retry once after 1 s, then
//! Err — NEVER negative-cached (transient).

use std::sync::Arc;
use std::time::Duration;

use super::lrclib_types::{LrclibRecord, LyricsPayload, LyricsQuery};
use super::rate_limit::RateLimitQueue;
use crate::db::lyrics_cache;
use crate::resolver::{fnv1a_hex, normalize_key_part, BoxFuture, SharedDb};

/// Sent as `Lrclib-Client` (their trace layer + CORS explicitly support it).
pub const CLIENT_ID: &str = "Adios/0.1 (personal use)";

/// FNV-1a over normalized artist|title|album|duration_s — same family as
/// the MB/CAA keys so case/spacing variants share one cache row.
pub fn cache_key(q: &LyricsQuery) -> String {
    let album = q.album.as_deref().unwrap_or("");
    let dur_s = q
        .duration_ms
        .map(|ms| (ms / 1000).to_string())
        .unwrap_or_default();
    fnv1a_hex(&format!(
        "lrclib|{}|{}|{}|{}",
        normalize_key_part(&q.artist),
        normalize_key_part(&q.title),
        normalize_key_part(album),
        dur_s
    ))
}

/// `https://lrclib.net/api/get?artist_name=&track_name=[&album_name=]
/// [&duration=]` — duration in whole seconds; `with_duration=false` is the
/// fallback used when a duration-qualified miss would otherwise 404.
pub fn build_url(q: &LyricsQuery, with_duration: bool) -> Result<String, String> {
    let mut url = reqwest::Url::parse("https://lrclib.net/api/get")
        .map_err(|e| format!("lrclib: bad base url: {e}"))?;
    let mut pairs = url.query_pairs_mut();
    pairs.append_pair("artist_name", q.artist.trim());
    pairs.append_pair("track_name", q.title.trim());
    if let Some(album) = q.album.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        pairs.append_pair("album_name", album);
    }
    if let Some(ms) = q.duration_ms.filter(|_| with_duration) {
        pairs.append_pair("duration", &(ms / 1000).to_string());
    }
    drop(pairs);
    Ok(url.to_string())
}

/// 404 is a meaningful business answer here (unlike MB), so the transport
/// hands back the status + body instead of collapsing it into Err.
pub struct TransportResponse {
    pub status: u16,
    pub body: String,
}

pub trait LrclibTransport: Send + Sync + 'static {
    fn get<'a>(&'a self, url: &'a str) -> BoxFuture<'a, Result<TransportResponse, String>>;
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

impl LrclibTransport for ReqwestTransport {
    fn get<'a>(&'a self, url: &'a str) -> BoxFuture<'a, Result<TransportResponse, String>> {
        Box::pin(async move {
            let res = self
                .http
                .get(url)
                .header("Lrclib-Client", CLIENT_ID)
                .send()
                .await
                .map_err(|e| format!("lrclib: request failed: {e}"))?;
            let status = res.status().as_u16();
            let body = res
                .text()
                .await
                .map_err(|e| format!("lrclib: read failed: {e}"))?;
            Ok(TransportResponse { status, body })
        })
    }
}

pub struct LyricsClient<T: LrclibTransport> {
    transport: T,
    queue: Arc<RateLimitQueue>,
    db: SharedDb,
}

impl LyricsClient<ReqwestTransport> {
    /// Production wiring: reqwest + its own 1-req/s queue (never the MB one).
    pub fn with_default_transport(db: SharedDb) -> Self {
        Self::new(
            ReqwestTransport::default(),
            RateLimitQueue::one_per_second(),
            db,
        )
    }
}

impl<T: LrclibTransport> LyricsClient<T> {
    pub fn new(transport: T, queue: Arc<RateLimitQueue>, db: SharedDb) -> Self {
        Self {
            transport,
            queue,
            db,
        }
    }

    /// One URL, pacing + one 503 retry (Retry-After: 1) built in.
    /// `Ok(Some)` = record; `Ok(None)` = authoritative 404.
    async fn get_once(&self, url: &str) -> Result<Option<LrclibRecord>, String> {
        for attempt in 0..2u8 {
            self.queue.wait().await;
            println!("[lrclib] fetch url={url}");
            let res = self.transport.get(url).await?;
            match res.status {
                200 => {
                    let rec: LrclibRecord = serde_json::from_str(&res.body)
                        .map_err(|e| format!("lrclib: decode: {e}"))?;
                    return Ok(Some(rec));
                }
                404 => return Ok(None),
                503 if attempt == 0 => {
                    println!("[lrclib] overloaded (503), retrying in 1s");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                503 => return Err("lrclib: overloaded (503), not cached".to_owned()),
                other => return Err(format!("lrclib: HTTP {other}")),
            }
        }
        Err("lrclib: overloaded (503), not cached".to_owned())
    }

    /// Cache-first fetch. `Ok(Some)` = lyrics found (cache or network);
    /// `Ok(None)` = known-absent (negative row written on first network
    /// answer); `Err` = transient (transport/503/decode — nothing cached).
    pub async fn fetch(&self, q: &LyricsQuery) -> Result<Option<LyricsPayload>, String> {
        let key = cache_key(q);
        if let Some(row) = lyrics_cache::get(&self.db, &key)? {
            println!(
                "[lrclib] cache {} key={key}",
                if row.found { "hit" } else { "negative" }
            );
            return Ok(if row.found { Some(row) } else { None });
        }
        // Strict pass (duration filter per docs), then a durationless
        // fallback on 404 before declaring absence — our file durations can
        // drift a second from LRCLIB's index.
        let rec = match self.get_once(&build_url(q, true)?).await? {
            Some(r) => Some(r),
            None if q.duration_ms.is_some() => {
                println!("[lrclib] duration-filtered 404, retrying without duration");
                self.get_once(&build_url(q, false)?).await?
            }
            None => None,
        };
        match rec {
            Some(r) if r.has_lyrics() => {
                let payload = LyricsPayload::from_record(r, "network");
                if let Err(e) = lyrics_cache::put_hit(&self.db, &key, &payload) {
                    eprintln!("[lrclib] cache write skipped: {e}");
                }
                println!("[lrclib] stored hit key={key}");
                Ok(Some(payload))
            }
            Some(r) => {
                // 200 but instrumental/empty — negative, reason preserved.
                if let Err(e) = lyrics_cache::put_not_found(&self.db, &key, r.instrumental) {
                    eprintln!("[lrclib] cache write skipped: {e}");
                }
                println!("[lrclib] stored not_found (instrumental={}) key={key}", r.instrumental);
                Ok(None)
            }
            None => {
                if let Err(e) = lyrics_cache::put_not_found(&self.db, &key, false) {
                    eprintln!("[lrclib] cache write skipped: {e}");
                }
                println!("[lrclib] stored not_found key={key}");
                Ok(None)
            }
        }
    }
}
