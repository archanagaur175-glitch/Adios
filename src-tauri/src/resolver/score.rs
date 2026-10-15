//! Candidate scoring (architecture §3.3): duration ±3 s hard bound,
//! token-set title similarity, "official audio"/"topic" hints, and
//! live/cover/remix rejection unless the query itself asks for it.

use std::collections::HashSet;

use super::Query;

/// Hard reject when the duration delta exceeds this (architecture rule).
pub const DURATION_HARD_BOUND_MS: i64 = 3000;
/// Minimum token-set title overlap for an Accept.
pub const MIN_TITLE_OVERLAP: f32 = 0.34;
/// Minimum confidence a chain resolver should accept (LocalFile uses this).
pub const MIN_ACCEPT_CONFIDENCE: f32 = 0.5;

const TITLE_WEIGHT: f32 = 0.65;
const ARTIST_WEIGHT: f32 = 0.35;
const OFFICIAL_AUDIO_BOOST: f32 = 0.05;
const TOPIC_BOOST: f32 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    DurationMismatch { delta_ms: i64 },
    NoTitleMatch,
    Live,
    Cover,
    Remix,
}

impl std::fmt::Display for RejectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RejectReason::DurationMismatch { delta_ms } => {
                write!(f, "duration delta {delta_ms}ms exceeds bound")
            }
            RejectReason::NoTitleMatch => write!(f, "title tokens do not overlap"),
            RejectReason::Live => write!(f, "live version rejected"),
            RejectReason::Cover => write!(f, "cover version rejected"),
            RejectReason::Remix => write!(f, "remix rejected"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoreOutcome {
    Reject(RejectReason),
    Accept { confidence: f32 },
}

/// Lowercase, strip punctuation to spaces, split into tokens.
pub fn normalize_tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Overlap coefficient over token sets; neutral 0.5 when either side is empty.
fn token_overlap(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.5;
    }
    let set_a: HashSet<&str> = a.iter().map(String::as_str).collect();
    let set_b: HashSet<&str> = b.iter().map(String::as_str).collect();
    let intersection = set_a.intersection(&set_b).count();
    let min_len = set_a.len().min(set_b.len());
    intersection as f32 / min_len as f32
}

/// Banned flavor words: rejected only when the QUERY does not contain them.
fn flavor_reject(candidate_tokens: &[String], query_tokens: &[String]) -> Option<RejectReason> {
    const BANNED: &[(&str, RejectReason)] = &[
        ("live", RejectReason::Live),
        ("cover", RejectReason::Cover),
        ("covers", RejectReason::Cover),
        ("remix", RejectReason::Remix),
        ("remixed", RejectReason::Remix),
    ];
    for (word, reason) in BANNED {
        if candidate_tokens.iter().any(|t| t == word)
            && !query_tokens.iter().any(|t| t == word)
        {
            return Some(*reason);
        }
    }
    None
}

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// Score one candidate against the query. Deterministic and pure (unit-tested).
pub fn score(
    query: &Query,
    candidate_title: &str,
    candidate_artist: &str,
    candidate_duration_ms: Option<i64>,
) -> ScoreOutcome {
    let query_title = normalize_tokens(&query.title);
    let query_artist = normalize_tokens(&query.artist);
    let cand_title = normalize_tokens(candidate_title);
    let cand_artist = normalize_tokens(candidate_artist);

    if let Some(reason) = flavor_reject(&cand_title, &query_title) {
        return ScoreOutcome::Reject(reason);
    }

    if let (Some(q), Some(c)) = (query.duration_ms, candidate_duration_ms) {
        let delta = (c - q).abs();
        if delta > DURATION_HARD_BOUND_MS {
            return ScoreOutcome::Reject(RejectReason::DurationMismatch { delta_ms: delta });
        }
    }

    let title_overlap = token_overlap(&query_title, &cand_title);
    if title_overlap < MIN_TITLE_OVERLAP {
        return ScoreOutcome::Reject(RejectReason::NoTitleMatch);
    }

    let artist_overlap = token_overlap(&query_artist, &cand_artist);
    let mut confidence = TITLE_WEIGHT * title_overlap + ARTIST_WEIGHT * artist_overlap;

    let raw = format!("{candidate_title} {candidate_artist}").to_lowercase();
    if raw.contains("official audio") {
        confidence += OFFICIAL_AUDIO_BOOST;
    }
    if cand_artist.last().is_some_and(|t| t == "topic") {
        confidence += TOPIC_BOOST;
    }

    ScoreOutcome::Accept {
        confidence: clamp01(confidence),
    }
}
