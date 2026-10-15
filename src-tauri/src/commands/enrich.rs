//! Import enrichment (P5.4): backfill MusicBrainz recording/release IDs
//! and CAA cover art for tracks still missing a match. One MB queue slot
//! per unmatched row (cache-first — repeats are meta_cache-only), CAA has
//! no rate limits (docs). No-match rows are never written to.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::db::{track_mb, DbState};
use crate::meta::cover_art::CoverArtClient;
use crate::meta::cover_store;
use crate::meta::enrich::{self, TrackMeta};
use crate::meta::mb_client::{MbClient, ReqwestTransport};

/// Single flight: a scan-triggered run and a manual button press must not
/// double-spend MB queue slots. Reset on drop, including error paths.
static RUNNING: AtomicBool = AtomicBool::new(false);

struct BusyGuard;
impl Drop for BusyGuard {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::SeqCst);
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichSummary {
    pub matched: i64,
    pub no_match: i64,
    pub failed: i64,
    pub covers: i64,
    pub elapsed_ms: i64,
}

/// Enrich unmatched rows (oldest first, ≤ 50 per call — the webview loops
/// while the batch is full). Idempotent: matched rows drop out of the
/// candidate set; no-match rows stay untouched but their MB search is
/// meta_cache-served on the next run.
#[tauri::command]
pub async fn enrich_tracks(
    app: AppHandle,
    mb: State<'_, MbClient<ReqwestTransport>>,
    cover: State<'_, CoverArtClient>,
    db: State<'_, DbState>,
) -> Result<EnrichSummary, String> {
    if RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("enrichment already running".to_owned());
    }
    let _busy = BusyGuard;

    let started = Instant::now();
    let mut out = EnrichSummary::default();
    let covers_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("covers");
    let shared = db.shared();

    let rows = {
        let conn = shared.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
        track_mb::unmatched(&conn, 50).map_err(|e| format!("db read failed: {e}"))?
    };

    for row in rows {
        let meta = TrackMeta {
            title: row.title.clone(),
            artist: row.artist.clone(),
            duration_ms: row.duration_ms,
        };
        match enrich::match_track(&mb, &meta).await {
            Ok(None) => {
                out.no_match += 1;
                println!("[enrich] no-match id={} title=\"{}\"", row.id, row.title);
            }
            Ok(Some(found)) => {
                {
                    let conn =
                        shared.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
                    track_mb::set_mb_ids(&conn, row.id, &found.recording_id, found.release_id.as_deref())
                        .map_err(|e| format!("db write failed: {e}"))?;
                }
                out.matched += 1;
                println!(
                    "[enrich] matched id={} rec={} rel={}",
                    row.id,
                    found.recording_id,
                    found.release_id.as_deref().unwrap_or("-")
                );
                if let Some(rel) = &found.release_id {
                    match cover_store::fetch_and_cache(&cover, &shared, &covers_dir, row.id, rel).await
                    {
                        Ok(_) => out.covers += 1,
                        Err(err) => {
                            out.failed += 1;
                            eprintln!("[enrich] cover failed id={} {err}", row.id);
                        }
                    }
                }
            }
            Err(err) => {
                out.failed += 1;
                eprintln!("[enrich] search failed id={} {err}", row.id);
            }
        }
    }

    out.elapsed_ms = started.elapsed().as_millis() as i64;
    println!(
        "[enrich] done matched={} no_match={} covers={} failed={} in {}ms",
        out.matched, out.no_match, out.covers, out.failed, out.elapsed_ms
    );
    Ok(out)
}
