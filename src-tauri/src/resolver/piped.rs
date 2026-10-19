//! Piped/Invidious fallback resolver (architecture §3.3, P4.4): last in the
//! chain when yt-dlp misses. EMPTY instance list ships by default (D4) —
//! instances come from settings key `pipedInstances`, never hard-coded.
//! Wire shapes + parsers live in `piped_api.rs` (verified live 2026-10-15).
use std::time::Instant;

use super::piped_api::{
    encode, parse_invidious_search, parse_piped_search, probe, SearchHit, Flavor, USER_AGENT,
};
use super::piped_formats::{parse_invidious_video, parse_piped_streams};
use super::score::{score, ScoreOutcome};
use super::{BoxFuture, Candidate, Health, HealthStatus, Query, ResolveError, Resolver, SharedDb};

const SETTINGS_KEY: &str = "pipedInstances";

pub struct PipedResolver {
    db: SharedDb,
    client: reqwest::Client,
}

impl PipedResolver {
    pub fn new(db: SharedDb) -> Self {
        Self {
            db,
            client: crate::net::http_client(),
        }
    }

    /// Settings-stored instance URLs; empty by default (D4).
    fn instances(&self) -> Vec<String> {
        let json: Option<String> = self.db.lock().ok().and_then(|conn| {
            conn.query_row(
                "SELECT value_json FROM settings WHERE key = ?1",
                [SETTINGS_KEY],
                |row| row.get(0),
            )
            .ok()
        });
        json.and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default()
    }

    async fn get_text(&self, url: &str) -> Result<String, ResolveError> {
        let res = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|e| ResolveError::Internal(format!("request failed: {e}")))?;
        if !res.status().is_success() {
            return Err(ResolveError::Internal(format!("status {}", res.status())));
        }
        res.text()
            .await
            .map_err(|e| ResolveError::Internal(format!("body failed: {e}")))
    }

    async fn search_hits(&self, base: &str, flavor: Flavor, term: &str) -> Vec<SearchHit> {
        let body = match flavor {
            Flavor::Piped => format!("{base}/search?q={}&filter=videos", encode(term)),
            Flavor::Invidious => format!("{base}/api/v1/search?q={}&type=video", encode(term)),
        };
        match self.get_text(&body).await {
            Ok(text) if flavor == Flavor::Piped => parse_piped_search(&text),
            Ok(text) => parse_invidious_search(&text),
            Err(err) => {
                println!("[resolver] piped search {base} -> miss: {err}");
                Vec::new()
            }
        }
    }

    async fn audio_stream(&self, base: &str, flavor: Flavor, id: &str) -> Option<(String, String)> {
        let url = match flavor {
            Flavor::Piped => format!("{base}/streams/{id}"),
            Flavor::Invidious => format!("{base}/api/v1/videos/{id}"),
        };
        match self.get_text(&url).await {
            Ok(text) if flavor == Flavor::Piped => parse_piped_streams(&text),
            Ok(text) => parse_invidious_video(&text),
            Err(err) => {
                println!("[resolver] piped streams {base} -> miss: {err}");
                None
            }
        }
    }

    async fn resolve_from(&self, base: &str, query: &Query) -> Result<Candidate, ResolveError> {
        let flavor = probe(&self.client, base)
            .await
            .ok_or_else(|| ResolveError::Internal(format!("instance down: {base}")))?;

        let term = format!("{} {}", query.artist, query.title);
        let mut best: Option<(SearchHit, f32)> = None;
        for hit in self.search_hits(base, flavor, &term).await {
            if let ScoreOutcome::Accept { confidence } =
                score(query, &hit.title, &hit.uploader, hit.duration_ms)
            {
                if best.as_ref().is_none_or(|(_, c)| confidence > *c) {
                    best = Some((hit, confidence));
                }
            }
        }
        let (hit, confidence) = best.ok_or(ResolveError::NotFound)?;

        let (stream_url, mime) = self
            .audio_stream(base, flavor, &hit.id)
            .await
            .ok_or(ResolveError::NotFound)?;
        Ok(Candidate {
            url_or_path: stream_url,
            mime: Some(mime),
            expires_at: None,
            source: "piped".to_owned(),
            confidence,
        })
    }
}

impl Resolver for PipedResolver {
    fn name(&self) -> &'static str {
        "piped"
    }

    fn health<'a>(&'a self) -> BoxFuture<'a, Health> {
        Box::pin(async move {
            let started = Instant::now();
            let instances = self.instances();
            let status = if instances.is_empty() {
                HealthStatus::Down // empty list ships by default (D4)
            } else if probe(&self.client, &instances[0]).await.is_some() {
                HealthStatus::Ok
            } else {
                HealthStatus::Down
            };
            Health {
                status,
                latency_ms: started.elapsed().as_millis() as u32,
            }
        })
    }

    fn resolve<'a>(&'a self, query: &'a Query) -> BoxFuture<'a, Result<Candidate, ResolveError>> {
        Box::pin(async move {
            let instances = self.instances();
            if instances.is_empty() {
                return Err(ResolveError::NotFound);
            }
            let mut last = ResolveError::NotFound;
            for base in instances {
                match self.resolve_from(&base, query).await {
                    Ok(candidate) => return Ok(candidate),
                    Err(err) => {
                        println!("[resolver] piped {base} -> miss: {err}");
                        last = err;
                    }
                }
            }
            Err(last)
        })
    }
}
