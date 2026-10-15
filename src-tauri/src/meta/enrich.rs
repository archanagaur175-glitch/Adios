//! Import enrichment (P5.4): match a local track against MusicBrainz
//! recording search results — title similarity + duration within ±3 s +
//! artist-credit disambiguation — best-of ranking, no write on no-match.
//!
//! Query syntax verified live 2026-10-15 (Rule 3): `recording:"<title>" AND
//! artist:"<artist>"` → HTTP 200 with score=100 hits
//! (`recording:"creep" AND artist:"radiohead"`, count=194).

use super::mb_client::{MbClient, MbTransport};
use super::mb_types::{MbRecording, MbReleaseRef, SearchKind};

/// Hard duration window: a candidate whose MB `length` differs from the
/// local file by more than this many milliseconds is rejected outright.
pub const DURATION_WINDOW_MS: i64 = 3000;
/// Below this title similarity a candidate is never considered.
pub const TITLE_MIN: f64 = 0.75;
/// Final ranking must reach this total (see `score_candidate`) to accept.
pub const ACCEPT_MIN: f64 = 2.5;

/// Normalization shared by every comparison: lowercase, alphanumeric kept,
/// everything else folded to spaces, whitespace collapsed. Diacritics stay
/// as-is — both sides get the same transform, so equal titles still compare
/// equal (P5.4 requires case/punctuation insensitivity only).
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = true;
    for ch in s.to_lowercase().chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim().to_owned()
}

/// Sørensen–Dice coefficient over character bigrams of the normalized
/// strings: 1.0 identical, 0.0 disjoint (multiset intersection, so repeated
/// bigrams cannot be counted twice). Deterministic, dependency-free.
pub fn title_similarity(a: &str, b: &str) -> f64 {
    let (na, nb) = (normalize(a), normalize(b));
    if na == nb {
        return 1.0;
    }
    let bigrams = |s: &str| -> Vec<String> {
        let chars: Vec<char> = s.chars().collect();
        chars.windows(2).map(|w| w.iter().collect()).collect()
    };
    let (mut ga, gb) = (bigrams(&na), bigrams(&nb));
    if ga.is_empty() || gb.is_empty() {
        return 0.0;
    }
    let mut shared = 0usize;
    for g in &gb {
        if let Some(pos) = ga.iter().position(|x| x == g) {
            ga.remove(pos);
            shared += 1;
        }
    }
    2.0 * shared as f64 / (ga.len() as f64 + gb.len() as f64 + shared as f64)
}

/// The local row under matching (kept free of DB/tauri types for tests).
#[derive(Debug, Clone)]
pub struct TrackMeta {
    pub title: String,
    pub artist: String,
    pub duration_ms: Option<i64>,
}

/// Score one MB candidate, or `None` = hard reject.
///
/// Components: `2×title_sim` (needs ≥ `TITLE_MIN`), `+1` when both durations
/// exist and fall inside `DURATION_WINDOW_MS` (a Δ beyond it rejects),
/// `+artist points` (exact 1.0 / partial-token 0.6 / neutral 0.5 when one
/// side is missing, reject on mismatch). Total must reach `ACCEPT_MIN`.
pub fn score_candidate(track: &TrackMeta, cand: &MbRecording) -> Option<f64> {
    let sim = title_similarity(&track.title, &cand.title);
    if sim < TITLE_MIN {
        return None;
    }
    let dur = match (track.duration_ms, cand.length) {
        (Some(local), Some(mb)) => {
            if (local - mb).abs() > DURATION_WINDOW_MS {
                return None;
            }
            1.0
        }
        _ => 0.0,
    };
    let artist = artist_points(&track.artist, &cand.artists())?;
    let total = 2.0 * sim + dur + artist;
    (total >= ACCEPT_MIN).then_some(total)
}

/// Artist-credit gate: exact 1.0, shared meaningful token 0.6, no signal
/// (either side empty) 0.5, complete mismatch = `None` (reject).
fn artist_points(track_artist: &str, credits: &[String]) -> Option<f64> {
    let track_norm = normalize(track_artist);
    let credit_norm: Vec<String> = credits.iter().map(|c| normalize(c)).collect();
    if track_norm.is_empty() || credit_norm.iter().all(|c| c.is_empty()) {
        return Some(0.5);
    }
    if credit_norm.iter().any(|c| *c == track_norm) {
        return Some(1.0);
    }
    let tokens: Vec<&str> = track_norm.split_whitespace().filter(|w| w.len() >= 3).collect();
    if tokens.is_empty() {
        return Some(0.5);
    }
    let overlap = tokens
        .iter()
        .any(|t| credit_norm.iter().any(|c| c.split_whitespace().any(|w| w == *t)));
    overlap.then_some(0.6)
}

/// Best candidate at or above `ACCEPT_MIN`; `None` when nothing qualifies
/// (the caller then leaves the row untouched — P5.4 requirement).
pub fn pick_match<'a>(track: &TrackMeta, cands: &'a [MbRecording]) -> Option<&'a MbRecording> {
    let mut best: Option<(&'a MbRecording, f64)> = None;
    for cand in cands {
        if let Some(score) = score_candidate(track, cand) {
            if best.map(|(_, s)| score > s).unwrap_or(true) {
                best = Some((cand, score));
            }
        }
    }
    match best {
        Some((cand, score)) if score >= ACCEPT_MIN => Some(cand),
        _ => None,
    }
}

/// Release whose front cover we fetch: Official first, then earliest date
/// (unknown dates sort last). Ties keep list order (`min_by` = first).
pub fn pick_release(releases: &[MbReleaseRef]) -> Option<&MbReleaseRef> {
    let rank = |r: &MbReleaseRef| u8::from(r.status.as_deref() != Some("Official"));
    let date_key = |r: &MbReleaseRef| r.date.clone().unwrap_or_else(|| "9999-99-99".into());
    releases
        .iter()
        .filter(|r| !r.id.trim().is_empty())
        .min_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| date_key(a).cmp(&date_key(b))))
}

/// Fielded MB query (double quotes in user text are folded away so they
/// cannot break the Lucene phrase; casing/punctuation inside the phrase is
/// left alone — the MB analyzers handle it). Empty artist → title only.
pub fn enrich_query(title: &str, artist: &str) -> String {
    let t = title.replace('"', " ");
    let a = artist.replace('"', " ");
    if a.trim().is_empty() {
        format!("recording:\"{t}\"")
    } else {
        format!("recording:\"{t}\" AND artist:\"{}\"", a.trim())
    }
}

/// The winning pair handed to persistence (P5.4 command writes both,
/// then fetches the release's CAA front cover).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MbMatch {
    pub recording_id: String,
    pub release_id: Option<String>,
}

/// Cache-first search + pick: one MB queue slot on a miss, zero on a
/// meta_cache hit. `Ok(None)` = nothing matched → row stays untouched.
pub async fn match_track<T: MbTransport>(
    mb: &MbClient<T>,
    track: &TrackMeta,
) -> Result<Option<MbMatch>, String> {
    let page = mb
        .search::<MbRecording>(SearchKind::Recording, &enrich_query(&track.title, &track.artist))
        .await?;
    let hit = pick_match(track, &page.items).cloned();
    Ok(hit.map(|rec| {
        let release = pick_release(rec.releases.as_deref().unwrap_or_default())
            .map(|r| r.id.clone());
        MbMatch {
            recording_id: rec.id,
            release_id: release,
        }
    }))
}
