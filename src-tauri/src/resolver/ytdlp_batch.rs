//! P20/D34: batched yt-dlp resolution — ONE process per chunk of queries.
//! Empirically verified (2026-10-19, yt-dlp 2026.08.19): N `ytsearch3:`
//! inputs + `--dump-single-json` print N newline-separated JSON docs, each
//! echoing the query term in its top-level `id` ("<artist> - <title>") with
//! the video results in `entries` — so docs map back to queries by that key
//! (FIFO for duplicates), with positional fallback when counts match.
//! Spawn failure = `latched: true`; the FE then stops batching and every
//! remaining pick goes through today's single-chain path (Android and a
//! yt-dlp-less desktop behave exactly as before).

use std::collections::HashMap;
use std::path::Path;

use super::ytdlp::{pick_candidate, pick_entries, sidecar_command};
use super::ytdlp_args::{search_term, FORMAT_SELECTOR, SOCKET_TIMEOUT_SECS};
use super::{Candidate, Query};

/// Per-query outcome, index-aligned with the request.
pub enum BatchSlot {
    Hit(Candidate),
    Miss(String),
}

pub struct BatchOutcome {
    /// True only when the PROCESS could not start — the FE latches off.
    pub latched: bool,
    pub slots: Vec<BatchSlot>,
}

/// One argv for the whole chunk: shared flags repeated per input (yt-dlp
/// accepts repeated flags; `--no-playlist` stays per-input semantics-safe —
/// verified live that search containers still emit full `entries`).
pub fn build_batch_args(queries: &[Query]) -> Vec<String> {
    let mut args = Vec::with_capacity(queries.len() * 9);
    for q in queries {
        args.extend([
            "--no-playlist".to_owned(),
            "--no-warnings".to_owned(),
            "--skip-download".to_owned(),
            "--socket-timeout".to_owned(),
            SOCKET_TIMEOUT_SECS.to_owned(),
            "--dump-single-json".to_owned(),
            "-f".to_owned(),
            FORMAT_SELECTOR.to_owned(),
            format!("ytsearch3:{}", search_term(q)),
        ]);
    }
    args
}

fn top_id(doc: &serde_json::Value) -> Option<&str> {
    doc.get("id").and_then(|v| v.as_str())
}

/// Map stdout docs back to `queries` (by echoed search term, FIFO for
/// duplicates; positional fallback when the counts match). Pure + tested.
pub fn parse_batch(stdout: &str, queries: &[Query]) -> Vec<BatchSlot> {
    let mut docs: Vec<serde_json::Value> = Vec::new();
    let mut stream = serde_json::Deserializer::from_str(stdout).into_iter::<serde_json::Value>();
    while let Some(Ok(doc)) = stream.next() {
        docs.push(doc);
    }

    let mut by_key: HashMap<String, Vec<usize>> = HashMap::new();
    for (di, doc) in docs.iter().enumerate() {
        if let Some(id) = top_id(doc) {
            by_key.entry(id.to_owned()).or_default().push(di);
        }
    }

    let mut doc_for: Vec<Option<usize>> = vec![None; queries.len()];
    let mut leftovers: Vec<usize> = Vec::new();
    for (qi, q) in queries.iter().enumerate() {
        match by_key.get_mut(&search_term(q)).and_then(|v| v.pop()) {
            Some(di) => doc_for[qi] = Some(di),
            None => leftovers.push(qi),
        }
    }
    // Positional fallback: same counts but ids no longer echo (yt-dlp shape
    // drift) — zip the unmatched docs with the unmatched queries in order.
    let unmatched_docs: Vec<usize> = (0..docs.len())
        .filter(|di| !doc_for.iter().any(|slot| slot == &Some(*di)))
        .collect();
    if unmatched_docs.len() == leftovers.len() {
        for (di, qi) in unmatched_docs.into_iter().zip(leftovers.iter()) {
            doc_for[*qi] = Some(di);
        }
        leftovers.clear();
    }

    let mut slots = Vec::with_capacity(queries.len());
    for (qi, q) in queries.iter().enumerate() {
        slots.push(match doc_for[qi] {
            Some(di) => match pick_candidate(pick_entries(&docs[di].to_string()), q) {
                Ok(c) => BatchSlot::Hit(c),
                Err(e) => BatchSlot::Miss(e.to_string()),
            },
            None => BatchSlot::Miss(format!(
                "batch: no result doc for \"{}\"",
                search_term(q)
            )),
        });
    }
    slots
}

/// Spawn ONE yt-dlp for the chunk. Non-zero exit still parses whatever docs
/// were emitted (yt-dlp exits non-zero if ANY input failed) — only a spawn
/// error latches the FE off batching.
pub async fn run_batch(path: &Path, queries: &[Query]) -> BatchOutcome {
    if queries.is_empty() {
        return BatchOutcome { latched: false, slots: Vec::new() };
    }
    let output = match sidecar_command(path).args(&build_batch_args(queries)).output().await {
        Ok(o) => o,
        Err(e) => {
            let msg = format!("yt-dlp spawn failed: {e}");
            return BatchOutcome {
                latched: true,
                slots: queries.iter().map(|_| BatchSlot::Miss(msg.clone())).collect(),
            };
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        println!(
            "[resolver] ytdlp batch exited {} with {} doc bytes (kept)",
            output.status.code().unwrap_or(-1),
            stdout.len()
        );
    }
    BatchOutcome { latched: false, slots: parse_batch(&stdout, queries) }
}
