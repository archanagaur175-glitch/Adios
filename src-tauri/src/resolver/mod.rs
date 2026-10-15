//! Resolver chain (architecture §3.3, Invariant 2): one trait, chain order
//! LocalFile → Cache → YtDlpSidecar → Piped/Invidious (settings-configurable).
//! P4.1 ships LocalFile + Cache; yt-dlp/piped arrive in P4.2/P4.4.
//!
//! resolve()/health-through-chain are consumed by the P4.2+ commands; the
//! module keeps them now so the trait shape is stable — attribute removed
//! when commands/resolver.rs lands.
#![allow(dead_code)]

pub mod cache;
pub mod local;
pub mod score;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod chain_tests;

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

/// Boxed future keeps the trait object-safe for the heterogeneous chain.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What the user (or a library track) wants resolved to a playable source.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
}

/// A playable resolution result (architecture §3.3 Candidate).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Local filesystem path or remote URL (loopback/proxy decides transport).
    pub url_or_path: String,
    pub mime: Option<String>,
    /// Remote URL expiry (RFC3339) when the source is time-limited.
    pub expires_at: Option<String>,
    /// Producing resolver: "local" | "cache" | "ytdlp" | "piped".
    pub source: String,
    /// 0.0–1.0 from score::score(); cache hits are 1.0.
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Ok,
    Degraded,
    Down,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub status: HealthStatus,
    pub latency_ms: u32,
}

#[derive(Debug)]
pub enum ResolveError {
    NotFound,
    Database(String),
    Internal(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::NotFound => write!(f, "no candidate"),
            ResolveError::Database(e) => write!(f, "database error: {e}"),
            ResolveError::Internal(e) => write!(f, "internal error: {e}"),
        }
    }
}

impl std::error::Error for ResolveError {}

pub trait Resolver: Send + Sync {
    fn name(&self) -> &'static str;
    fn health<'a>(&'a self) -> BoxFuture<'a, Health>;
    fn resolve<'a>(&'a self, query: &'a Query) -> BoxFuture<'a, Result<Candidate, ResolveError>>;
}

/// Ordered resolver chain: first successful candidate wins; misses logged.
pub struct ResolverChain {
    resolvers: Vec<Arc<dyn Resolver>>,
}

/// Shared chain managed by Tauri state.
pub type SharedChain = Arc<ResolverChain>;

impl ResolverChain {
    pub fn new(resolvers: Vec<Arc<dyn Resolver>>) -> Self {
        Self { resolvers }
    }

    pub async fn resolve(&self, query: &Query) -> Result<Candidate, ResolveError> {
        let mut last_err = ResolveError::NotFound;
        for resolver in &self.resolvers {
            match resolver.resolve(query).await {
                Ok(candidate) => {
                    println!(
                        "[resolver] {} -> hit conf={:.2} src={}",
                        resolver.name(),
                        candidate.confidence,
                        candidate.url_or_path
                    );
                    return Ok(candidate);
                }
                Err(err) => {
                    println!("[resolver] {} -> miss: {err}", resolver.name());
                    last_err = err;
                }
            }
        }
        Err(last_err)
    }

    pub async fn health(&self) -> Vec<(&'static str, Health)> {
        let mut out = Vec::with_capacity(self.resolvers.len());
        for resolver in &self.resolvers {
            out.push((resolver.name(), resolver.health().await));
        }
        out
    }
}

/// Shared DB handle used by the DB-backed resolvers.
pub type SharedDb = Arc<Mutex<Connection>>;

/// FNV-1a 64-bit hex — same algorithm as scan cover naming.
pub fn fnv1a_hex(input: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in input.bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Normalized single-spaced text — cache keys share score's normalization.
pub fn normalize_key_part(text: &str) -> String {
    score::normalize_tokens(text).join(" ")
}
