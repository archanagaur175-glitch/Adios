//! P19 (D33): Android yt-dlp bridge commands. `resolve_android_ping` is
//! the 19.1 skeleton proof; `resolve_android` (19.2) runs the Kotlin
//! search+score+format pipeline and returns the shared Candidate shape
//! so the FE adapter (19.3) can route mobile resolves transparently;
//! `resolve_android_batch` (19.4) mirrors `resolve_batch` end-to-end —
//! same `{latched, items}` IPC shape and per-hit write-back.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::resolver::BatchOutcomeOut;
#[cfg(target_os = "android")]
use crate::commands::resolver::BatchItem;
#[cfg(target_os = "android")]
use crate::resolver::cache;
use crate::resolver::mobile::YtDlpMobile;
use crate::resolver::{Candidate, Query, SharedTable};
use crate::db::DbState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingOut {
    pub ok: bool,
    pub version: Option<String>,
    pub error: Option<String>,
}

/// Kotlin `resolve` payload — mapped onto the shared Candidate below.
#[cfg(target_os = "android")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AndroidResolveOut {
    url: String,
    mime: Option<String>,
    confidence: f32,
    source: String,
}

#[tauri::command]
pub async fn resolve_android_ping(mobile: State<'_, YtDlpMobile>) -> Result<PingOut, String> {
    #[cfg(target_os = "android")]
    {
        mobile
            .0
            .run_mobile_plugin_async("ping", serde_json::json!({}))
            .await
            .map_err(|e| format!("ytdlp plugin: {e}"))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = mobile;
        Err("Android-only (desktop uses the Rust chain)".into())
    }
}

#[tauri::command]
pub async fn resolve_android(
    mobile: State<'_, YtDlpMobile>,
    artist: String,
    title: String,
    album: Option<String>,
    duration_ms: Option<i64>,
) -> Result<Candidate, String> {
    #[cfg(target_os = "android")]
    {
        let out: AndroidResolveOut = mobile
            .0
            .run_mobile_plugin_async(
                "resolve",
                serde_json::json!({
                    "artist": artist,
                    "title": title,
                    "album": album,
                    "durationMs": duration_ms,
                }),
            )
            .await
            .map_err(|e| format!("ytdlp resolve: {e}"))?;
        Ok(Candidate {
            url_or_path: out.url,
            mime: out.mime,
            expires_at: None,
            source: out.source,
            confidence: out.confidence,
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (mobile, artist, title, album, duration_ms);
        Err("Android-only (desktop uses the Rust chain)".into())
    }
}

/// Kotlin `resolveBatch` items — mapped onto the shared BatchItem below.
#[cfg(target_os = "android")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AndroidBatchItemOut {
    ok: bool,
    url: Option<String>,
    mime: Option<String>,
    confidence: Option<f32>,
    source: Option<String>,
    error: Option<String>,
}

#[cfg(target_os = "android")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AndroidBatchOut {
    latched: bool,
    items: Vec<AndroidBatchItemOut>,
}

/// P19.4: batched Android resolution — one plugin call per chunk with the
/// same payload/`{latched, items}` shape as desktop `resolve_batch`, plus
/// the same per-hit write-back (resolution table + meta_cache; a cache-write
/// failure keeps the candidate — a playable hit beats bookkeeping). Kotlin
/// returns `latched: true` when the plugin/process could not run, so the FE
/// stops batching and every pick falls back to the sequential chain.
#[tauri::command]
pub async fn resolve_android_batch(
    db: State<'_, DbState>,
    table: State<'_, SharedTable>,
    mobile: State<'_, YtDlpMobile>,
    queries: Vec<Query>,
) -> Result<BatchOutcomeOut, String> {
    #[cfg(target_os = "android")]
    {
        let payload = serde_json::json!({
            "queries": queries.iter().map(|q| serde_json::json!({
                "artist": q.artist,
                "title": q.title,
                "album": q.album,
                "durationMs": q.duration_ms,
            })).collect::<Vec<_>>(),
        });
        let out: AndroidBatchOut = mobile
            .0
            .run_mobile_plugin_async("resolveBatch", payload)
            .await
            .map_err(|e| format!("ytdlp batch: {e}"))?;
        let mut items = Vec::with_capacity(queries.len());
        for (q, it) in queries.iter().zip(out.items) {
            items.push(match (it.ok, it.url) {
                (true, Some(url)) => {
                    let candidate = Candidate {
                        url_or_path: url,
                        mime: it.mime,
                        expires_at: None,
                        source: it.source.unwrap_or_else(|| "ytdlp".into()),
                        confidence: it.confidence.unwrap_or(0.0),
                    };
                    if cache::is_remote_url(&candidate.url_or_path) {
                        table.register(&candidate.url_or_path, q);
                        if let Err(e) = cache::put_resolution(&db.shared(), q, &candidate) {
                            println!("[resolver] android batch write-back skipped: {e}");
                        }
                    }
                    BatchItem { ok: true, candidate: Some(candidate), error: None }
                }
                (_, url) => BatchItem {
                    ok: false,
                    candidate: None,
                    error: Some(it.error.unwrap_or_else(|| format!("missing url: {url:?}"))),
                },
            });
        }
        Ok(BatchOutcomeOut { latched: out.latched, items })
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (db, table, mobile, queries);
        Err("Android-only (desktop uses resolve_batch)".into())
    }
}
