//! Resolver commands (P4.4): manual query resolution for smoke/UI, chain
//! health, and instance testing (Settings "Test instance" button).

use std::time::Instant;

use serde::Serialize;
use tauri::State;

use crate::db::DbState;
use crate::resolver::piped_api::probe;
use crate::resolver::{cache, Candidate, HealthStatus, Query, SharedChain, SharedTable};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolverHealthOut {
    pub name: String,
    pub status: HealthStatus,
    pub latency_ms: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceHealth {
    pub ok: bool,
    pub flavor: Option<String>,
    pub latency_ms: u32,
    pub message: String,
}

/// Resolve one query through the configured chain (smoke + P5 UI entry).
/// Remote hits register in the resolution table + meta_cache (write-back).
#[tauri::command]
pub async fn resolve_query(
    db: State<'_, DbState>,
    chain: State<'_, SharedChain>,
    table: State<'_, SharedTable>,
    artist: String,
    title: String,
    album: Option<String>,
    duration_ms: Option<i64>,
) -> Result<Candidate, String> {
    let query = Query {
        artist,
        title,
        album,
        duration_ms,
    };
    let candidate = chain.resolve(&query).await.map_err(|e| e.to_string())?;
    if cache::is_remote_url(&candidate.url_or_path) {
        table.register(&candidate.url_or_path, &query);
        cache::put_resolution(&db.shared(), &query, &candidate)?;
    }
    Ok(candidate)
}

/// Per-resolver health sweep (Settings diagnostics).
#[tauri::command]
pub async fn resolver_health(chain: State<'_, SharedChain>) -> Result<Vec<ResolverHealthOut>, String> {
    let raw = chain.health().await;
    Ok(raw
        .into_iter()
        .map(|(name, health)| ResolverHealthOut {
            name: name.to_owned(),
            status: health.status,
            latency_ms: health.latency_ms,
        })
        .collect())
}

/// Probe one configured instance; never errors â€” reports `ok: false`.
#[tauri::command]
pub async fn test_instance(url: String) -> Result<InstanceHealth, String> {
    let base = url.trim_end_matches('/').to_owned();
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Ok(InstanceHealth {
            ok: false,
            flavor: None,
            latency_ms: 0,
            message: format!("not an http(s) url: {base}"),
        });
    }
    let started = Instant::now();
    let client = crate::net::http_client();
    match probe(&client, &base).await {
        Some(flavor) => Ok(InstanceHealth {
            ok: true,
            flavor: Some(
                match flavor {
                    crate::resolver::piped_api::Flavor::Piped => "piped",
                    crate::resolver::piped_api::Flavor::Invidious => "invidious",
                }
                .to_owned(),
            ),
            latency_ms: started.elapsed().as_millis() as u32,
            message: "instance responds with a supported API".to_owned(),
        }),
        None => Ok(InstanceHealth {
            ok: false,
            flavor: None,
            latency_ms: started.elapsed().as_millis() as u32,
            message: "no supported API found (Piped /suggestions or Invidious /api/v1/stats)"
                .to_owned(),
        }),
    }
}
