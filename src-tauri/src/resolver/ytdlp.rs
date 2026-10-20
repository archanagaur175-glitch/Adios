//! yt-dlp sidecar resolver (architecture §3.3, P4.2/P4.4): `ytsearch3:`
//! query → `--dump-single-json` → score entries → bestaudio stream URL.
//! Async tokio::process (P4.4) so a 5 s extraction never blocks a runtime
//! worker; the binary path lives in a shared `BinHandle` so the
//! "Update yt-dlp" command can hot-swap it. Arg construction lives in
//! ytdlp_args.rs (pure, 200-line budget).
//!
//! Spike (2026.08.19 binary, 2026-10-15): search + m4a format extraction
//! succeeded in ~5 s with NO EJS/JS-runtime and NO PO-token warnings.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use serde::Deserialize;
use tokio::process::Command;

use super::score::{score, ScoreOutcome};
use super::ytdlp_args::build_args;
use super::{BoxFuture, Candidate, Health, HealthStatus, Query, ResolveError, Resolver};

/// Shared yt-dlp binary path: resolver reads, `update_ytdlp` hot-swaps.
pub type BinHandle = Arc<tokio::sync::RwLock<PathBuf>>;

pub fn bin_handle(initial: PathBuf) -> BinHandle {
    Arc::new(tokio::sync::RwLock::new(initial))
}

/// Sidecar spawn helper: on Windows never flash a console window (user
/// report — picking a search track flashed cmd.exe). CREATE_NO_WINDOW.
pub fn sidecar_command(path: &std::path::Path) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut cmd = Command::new(path);
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW (tokio inherent)
    }
    cmd
}

pub struct YtDlpResolver {
    bin: BinHandle,
}

impl YtDlpResolver {
    pub fn new(bin: BinHandle) -> Self {
        Self { bin }
    }

    async fn health_async(&self) -> Health {
        let started = Instant::now();
        let path = self.bin.read().await.clone();
        let status = match sidecar_command(&path).arg("--version").output().await {
            Ok(out) if out.status.success() && !out.stdout.is_empty() => HealthStatus::Ok,
            _ => HealthStatus::Down,
        };
        Health {
            status,
            latency_ms: started.elapsed().as_millis() as u32,
        }
    }

    async fn resolve_async(&self, query: &Query) -> Result<Candidate, ResolveError> {
        let path = self.bin.read().await.clone();
        let args = build_args(query);
        let output = sidecar_command(&path)
            .args(&args)
            .output()
            .await
            .map_err(|e| ResolveError::Internal(format!("yt-dlp spawn failed: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ResolveError::Internal(format!(
                "yt-dlp exited {}: {}",
                output.status.code().unwrap_or(-1),
                stderr.chars().take(300).collect::<String>()
            )));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let entries = pick_entries(&stdout);
        pick_candidate(entries, query)
    }
}

/// P9.7: try each result in relevance order; first score() accept wins
/// (video #1 being deleted/region-blocked no longer fails the resolve).
/// P20: extracted from `resolve_async` so the batch path reuses it verbatim.
pub fn pick_candidate(entries: Vec<Entry>, query: &Query) -> Result<Candidate, ResolveError> {
    if entries.is_empty() {
        return Err(ResolveError::Internal("yt-dlp returned no results".to_owned()));
    }
    let mut first_reject: Option<String> = None;
    for entry in entries {
        let title = entry.title.clone().unwrap_or_default();
        let artist = entry
            .channel
            .clone()
            .or_else(|| entry.uploader.clone())
            .unwrap_or_default();
        let duration_ms = entry.duration.map(|secs| (secs * 1000.0).round() as i64);
        match score(query, &title, &artist, duration_ms) {
            ScoreOutcome::Reject(reason) => {
                if first_reject.is_none() {
                    first_reject = Some(format!("{reason} (\"{title}\")"));
                }
                println!("[resolver] ytdlp reject: {reason} (\"{title}\")");
            }
            ScoreOutcome::Accept { confidence } => {
                // Prefer the extracted stream URL; fall back to the page
                // URL so a later step can re-extract if it expired.
                let url = entry
                    .url
                    .or(entry.webpage_url)
                    .ok_or(ResolveError::NotFound)?;
                return Ok(Candidate {
                    mime: entry.ext.as_deref().map(mime_for_audio_ext),
                    url_or_path: url,
                    expires_at: None, // googlevideo expiry is URL-param based (P4.4 re-resolves)
                    source: "ytdlp".to_owned(),
                    confidence,
                });
            }
        }
    }
    Err(ResolveError::Internal(format!(
        "no usable YouTube video ({})",
        first_reject.unwrap_or_else(|| "unrated results".to_owned())
    )))
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub title: Option<String>,
    pub channel: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub ext: Option<String>,
    pub url: Option<String>,
    pub webpage_url: Option<String>,
}

/// ytsearch dumps `{entries:[...]}`; a direct URL dumps the entry at root.
/// P9.7: returns every entry (relevance order) so resolve can skip dead #1.
pub fn pick_entries(json: &str) -> Vec<Entry> {
    let value: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    if let Some(entries) = value.get("entries").and_then(|e| e.as_array()) {
        return entries
            .iter()
            .filter_map(|e| serde_json::from_value(e.clone()).ok())
            .collect();
    }
    match serde_json::from_value(value) {
        Ok(entry) => vec![entry],
        Err(_) => Vec::new(),
    }
}

fn mime_for_audio_ext(ext: &str) -> String {
    match ext {
        "m4a" | "mp4" => "audio/mp4",
        "webm" => "audio/webm",
        "mp3" => "audio/mpeg",
        other => other,
    }
    .to_owned()
}

impl Resolver for YtDlpResolver {
    fn name(&self) -> &'static str {
        "ytdlp"
    }

    fn health<'a>(&'a self) -> BoxFuture<'a, Health> {
        Box::pin(self.health_async())
    }

    fn resolve<'a>(&'a self, query: &'a Query) -> BoxFuture<'a, Result<Candidate, ResolveError>> {
        Box::pin(self.resolve_async(query))
    }
}
