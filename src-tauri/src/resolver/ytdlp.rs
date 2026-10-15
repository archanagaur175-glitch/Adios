//! yt-dlp sidecar resolver (architecture §3.3, P4.2): `ytsearch1:` query →
//! `--dump-single-json` → score the entry → bestaudio stream URL.
//!
//! Spike (2026.08.19 binary, 2026-10-15): search + m4a format extraction
//! succeeded in ~5 s with NO EJS/JS-runtime and NO PO-token warnings.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use serde::Deserialize;

use super::score::{score, ScoreOutcome};
use super::{BoxFuture, Candidate, Health, HealthStatus, Query, ResolveError, Resolver};

/// Prefer m4a (AAC); fall back to any bestaudio (webm/Opus plays natively).
const FORMAT_SELECTOR: &str = "bestaudio[ext=m4a]/bestaudio";
/// yt-dlp network bound per request — keeps a hung instance from stalling.
const SOCKET_TIMEOUT_SECS: &str = "15";

pub struct YtDlpResolver {
    bin: PathBuf,
}

impl YtDlpResolver {
    pub fn new(bin: PathBuf) -> Self {
        Self { bin }
    }

    fn health_blocking(&self) -> Health {
        let started = Instant::now();
        let status = match Command::new(&self.bin)
            .arg("--version")
            .output()
        {
            Ok(out) if out.status.success() && !out.stdout.is_empty() => HealthStatus::Ok,
            _ => HealthStatus::Down,
        };
        Health {
            status,
            latency_ms: started.elapsed().as_millis() as u32,
        }
    }

    fn resolve_blocking(&self, query: &Query) -> Result<Candidate, ResolveError> {
        let args = build_args(query);
        let output = Command::new(&self.bin)
            .args(&args)
            .output()
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
        let entry = pick_entry(&stdout).ok_or(ResolveError::NotFound)?;

        let title = entry.title.unwrap_or_default();
        let artist = entry
            .channel
            .or(entry.uploader)
            .unwrap_or_default();
        let duration_ms = entry.duration.map(|secs| (secs * 1000.0).round() as i64);

        match score(query, &title, &artist, duration_ms) {
            ScoreOutcome::Reject(reason) => {
                println!("[resolver] ytdlp reject: {reason} (\"{title}\")");
                Err(ResolveError::NotFound)
            }
            ScoreOutcome::Accept { confidence } => {
                // Prefer the extracted stream URL; fall back to the page URL
                // so a later step can re-extract if the direct URL expired.
                let url = entry
                    .url
                    .or(entry.webpage_url)
                    .ok_or(ResolveError::NotFound)?;
                Ok(Candidate {
                    mime: entry.ext.as_deref().map(mime_for_audio_ext),
                    url_or_path: url,
                    expires_at: None, // googlevideo expiry is URL-param based (P4.4 re-resolves)
                    source: "ytdlp".to_owned(),
                    confidence,
                })
            }
        }
    }
}

/// Pure arg builder — the spike command and unit tests share this.
pub fn build_args(query: &Query) -> Vec<String> {
    let search = format!("ytsearch1:{} - {}", query.artist, query.title);
    vec![
        "--no-playlist".to_owned(),
        "--no-warnings".to_owned(),
        "--skip-download".to_owned(),
        "--socket-timeout".to_owned(),
        SOCKET_TIMEOUT_SECS.to_owned(),
        "--dump-single-json".to_owned(),
        "-f".to_owned(),
        FORMAT_SELECTOR.to_owned(),
        search,
    ]
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
pub fn pick_entry(json: &str) -> Option<Entry> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    if let Some(entries) = value.get("entries").and_then(|e| e.as_array()) {
        return entries
            .first()
            .and_then(|e| serde_json::from_value(e.clone()).ok());
    }
    serde_json::from_value(value).ok()
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
        // Blocking process spawn; fine for personal-use scale (noted in docs).
        Box::pin(std::future::ready(self.health_blocking()))
    }

    fn resolve<'a>(&'a self, query: &'a Query) -> BoxFuture<'a, Result<Candidate, ResolveError>> {
        let outcome = self.resolve_blocking(query);
        Box::pin(std::future::ready(outcome))
    }
}
