//! Cache resolver: serves queries previously resolved by yt-dlp/piped from
//! meta_cache (query → URL) + cache_index (URL → disk file). P4.1 provides
//! the read path; cache_index writes landed in P4.3 (server/proxy tee), the
//! meta_cache resolution write-back arrives with P4.4.

use std::future::ready;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rusqlite::Connection;
use serde::Deserialize;

use super::{fnv1a_hex, normalize_key_part};
use super::{BoxFuture, Candidate, Health, HealthStatus, Query, ResolveError, Resolver, SharedDb};

const KIND_RESOLUTION: &str = "resolution";

#[derive(Debug, Deserialize)]
struct ResolutionPayload {
    url: String,
    mime: Option<String>,
    expires_at: Option<String>,
}

pub struct CacheResolver {
    db: SharedDb,
    cache_dir: PathBuf,
}

/// Deterministic cache key for a query: normalized tokens + duration seconds.
pub fn cache_key(query: &Query) -> String {
    let artist = normalize_key_part(&query.artist);
    let title = normalize_key_part(&query.title);
    let album = query
        .album
        .as_deref()
        .map(normalize_key_part)
        .unwrap_or_default();
    let duration_secs = query
        .duration_ms
        .map(|ms| (ms / 1000).to_string())
        .unwrap_or_default();
    fnv1a_hex(&format!("{artist}|{title}|{album}|{duration_secs}"))
}

impl CacheResolver {
    pub fn new(db: Arc<Mutex<Connection>>, cache_dir: PathBuf) -> Self {
        Self { db, cache_dir }
    }

    fn health_blocking(&self) -> Health {
        let started = Instant::now();
        let status = match self.db.lock() {
            Ok(conn) => match conn.query_row("SELECT 1", [], |_| Ok(())) {
                Ok(_) if self.cache_dir.is_dir() => HealthStatus::Ok,
                Ok(_) => HealthStatus::Degraded,
                Err(_) => HealthStatus::Down,
            },
            Err(_) => HealthStatus::Down,
        };
        Health {
            status,
            latency_ms: started.elapsed().as_millis() as u32,
        }
    }

    fn resolve_blocking(&self, query: &Query) -> Result<Candidate, ResolveError> {
        let key = cache_key(query);
        let conn = self.db.lock().map_err(|e| ResolveError::Internal(e.to_string()))?;

        let payload_json: String = match conn.query_row(
            "SELECT payload_json FROM meta_cache WHERE cache_key = ?1 AND kind = ?2",
            rusqlite::params![key, KIND_RESOLUTION],
            |row| row.get(0),
        ) {
            Ok(json) => json,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Err(ResolveError::NotFound),
            Err(e) => return Err(ResolveError::Database(e.to_string())),
        };
        let payload: ResolutionPayload = serde_json::from_str(&payload_json)
            .map_err(|e| ResolveError::Internal(format!("bad resolution payload: {e}")))?;

        let indexed: (String, Option<String>) = match conn.query_row(
            "SELECT local_path, mime FROM cache_index WHERE url = ?1",
            [&payload.url],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ) {
            Ok(row) => row,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Err(ResolveError::NotFound),
            Err(e) => return Err(ResolveError::Database(e.to_string())),
        };
        let (local_path, index_mime) = indexed;

        if std::fs::metadata(&local_path).is_err() {
            return Err(ResolveError::NotFound);
        }

        Ok(Candidate {
            url_or_path: local_path,
            mime: index_mime.or(payload.mime),
            expires_at: payload.expires_at,
            source: "cache".to_owned(),
            confidence: 1.0,
        })
    }
}

impl Resolver for CacheResolver {
    fn name(&self) -> &'static str {
        "cache"
    }

    fn health<'a>(&'a self) -> BoxFuture<'a, Health> {
        Box::pin(ready(self.health_blocking()))
    }

    fn resolve<'a>(&'a self, query: &'a Query) -> BoxFuture<'a, Result<Candidate, ResolveError>> {
        let outcome = self.resolve_blocking(query);
        Box::pin(ready(outcome))
    }
}
