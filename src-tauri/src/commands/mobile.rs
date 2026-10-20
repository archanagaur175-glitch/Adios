//! P19 (D33): Android yt-dlp bridge commands. `resolve_android_ping` is
//! the 19.1 skeleton proof; `resolve_android` (19.2) runs the Kotlin
//! search+score+format pipeline and returns the shared Candidate shape
//! so the FE adapter (19.3) can route mobile resolves transparently.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::resolver::mobile::YtDlpMobile;
use crate::resolver::Candidate;

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
