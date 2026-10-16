//! LRCLIB API types (P7.1). Shapes verified LIVE 2026-10-16 — see BUILD_PLAN
//! VERIFICATION LOG: `GET https://lrclib.net/api/get?artist_name=…&
//! track_name=…&album_name=…&duration=…` → 200
//! `{id, name, trackName, artistName, albumName, duration (float seconds),
//! instrumental, hasWordSync, plainLyrics, syncedLyrics, lyricsfile}`;
//! miss → 404 (burst traffic → 503 + Retry-After: 1, a Cloudflare limit —
//! never negative-cached). `/api/search` returns an array of the same item
//! shape (live-confirmed: 20 items, identical key set). `lyricsfile` is an
//! enhanced-LRC YAML variant we do not use. Official docs page is JS-
//! rendered; shapes were taken from these live responses + the server
//! source (github.com/tranxuanthang/lrclib — errors.rs, router.rs).

use serde::{Deserialize, Serialize};

/// One LRCLIB record (camelCase JSON, defaults so partial records never
/// hard-fail a decode). Fields the payload never surfaces (trackName,
/// artistName, albumName, duration) are kept for shape fidelity with the
/// live API — hence the dead_code allow.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LrclibRecord {
    pub id: i64,
    #[serde(default)]
    pub track_name: String,
    #[serde(default)]
    pub artist_name: String,
    #[serde(default)]
    pub album_name: Option<String>,
    /// Seconds (LRCLIB float), e.g. 261.0.
    #[serde(default)]
    pub duration: Option<f64>,
    #[serde(default)]
    pub instrumental: bool,
    #[serde(default)]
    pub plain_lyrics: Option<String>,
    #[serde(default)]
    pub synced_lyrics: Option<String>,
}

impl LrclibRecord {
    /// A 200 answer with neither plain nor synced text (instrumental or
    /// empty) counts as "no lyrics" → negative row, never a hit.
    pub fn has_lyrics(&self) -> bool {
        let nonempty = |s: &Option<String>| s.as_deref().is_some_and(|t| !t.trim().is_empty());
        nonempty(&self.plain_lyrics) || nonempty(&self.synced_lyrics)
    }
}

/// Normalized lookup identity for one track (cache key + URL build).
#[derive(Debug, Clone)]
pub struct LyricsQuery {
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration_ms: Option<i64>,
}

/// What the webview receives (and what a cache row maps to).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsPayload {
    pub found: bool,
    /// Raw LRC text (timestamps as `[mm:ss.xx]`); None when plain-only.
    pub synced: Option<String>,
    pub plain: Option<String>,
    pub instrumental: bool,
    pub lrclib_id: Option<i64>,
    /// "cache" | "network" | "none".
    pub source: String,
}

impl LyricsPayload {
    pub fn from_record(r: LrclibRecord, source: &str) -> Self {
        let clean = |s: Option<String>| s.filter(|t| !t.trim().is_empty());
        Self {
            found: true,
            synced: clean(r.synced_lyrics),
            plain: clean(r.plain_lyrics),
            instrumental: r.instrumental,
            lrclib_id: Some(r.id),
            source: source.to_owned(),
        }
    }

    /// Command-level answer when the cache/network says "known absent".
    pub fn not_found() -> Self {
        Self {
            found: false,
            synced: None,
            plain: None,
            instrumental: false,
            lrclib_id: None,
            source: "none".to_owned(),
        }
    }
}
