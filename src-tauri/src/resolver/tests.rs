//! P4.1 tests: scoring rules (required) + LocalFile resolver.

use std::sync::{Arc, Mutex};

use futures::FutureExt;

use crate::db::queries::{track_insert, NewTrack};
use crate::db::open_memory;
use crate::resolver::local::LocalFileResolver;
use crate::resolver::score::{score, RejectReason, ScoreOutcome};
use crate::resolver::{HealthStatus, Query, ResolveError, Resolver};

fn query(title: &str, artist: &str, duration_ms: Option<i64>) -> Query {
    Query {
        artist: artist.to_owned(),
        title: title.to_owned(),
        album: None,
        duration_ms,
    }
}

fn accept_confidence(outcome: ScoreOutcome) -> f32 {
    match outcome {
        ScoreOutcome::Accept { confidence } => confidence,
        other => panic!("expected Accept, got {other:?}"),
    }
}

fn reject_reason(outcome: ScoreOutcome) -> RejectReason {
    match outcome {
        ScoreOutcome::Reject(reason) => reason,
        other => panic!("expected Reject, got {other:?}"),
    }
}

// ---------- scoring (plan-required cases) ----------

#[test]
fn score_exact_match_is_high_confidence() {
    let q = query("Apna Bana Le", "Arijit Singh", Some(261_000));
    let c = score(&q, "Apna Bana Le", "Arijit Singh", Some(261_360));
    assert!(accept_confidence(c) >= 0.99, "got {c:?}");
}

#[test]
fn score_rejects_duration_beyond_hard_bound() {
    let q = query("Tum Hi Ho", "Arijit Singh", Some(262_000));
    let c = score(&q, "Tum Hi Ho", "Arijit Singh", Some(277_000));
    assert_eq!(
        reject_reason(c),
        RejectReason::DurationMismatch { delta_ms: 15_000 }
    );
}

#[test]
fn score_accepts_duration_within_bound() {
    let q = query("Tum Hi Ho", "Arijit Singh", Some(262_000));
    let c = score(&q, "Tum Hi Ho", "Arijit Singh", Some(264_000));
    assert!(matches!(c, ScoreOutcome::Accept { .. }), "got {c:?}");
}

#[test]
fn score_rejects_live_unless_query_is_live() {
    let plain = query("Channa Mereya", "Arijit Singh", None);
    let live_cand = score(&plain, "Channa Mereya (Live)", "Arijit Singh", None);
    assert_eq!(reject_reason(live_cand), RejectReason::Live);

    let live_q = query("Channa Mereya (Live)", "Arijit Singh", None);
    let live_ok = score(&live_q, "Channa Mereya (Live)", "Arijit Singh", None);
    assert!(matches!(live_ok, ScoreOutcome::Accept { .. }), "got {live_ok:?}");
}

#[test]
fn score_rejects_cover_and_remix() {
    let q = query("Kesariya", "Arijit Singh", None);
    assert_eq!(
        reject_reason(score(&q, "Kesariya (Cover)", "Some Singer", None)),
        RejectReason::Cover
    );
    assert_eq!(
        reject_reason(score(&q, "Kesariya (Remix)", "DJ X", None)),
        RejectReason::Remix
    );
}

#[test]
fn score_official_audio_hint_boosts_confidence() {
    let q = query("Apna Bana Le", "Arijit Singh", Some(261_000));
    // Artist mismatch keeps the baseline below the clamp so the boost shows.
    let plain = accept_confidence(score(&q, "Apna Bana Le", "Various Artists", Some(261_000)));
    let boosted = accept_confidence(score(
        &q,
        "Apna Bana Le (Official Audio)",
        "Various Artists",
        Some(261_000),
    ));
    assert!(boosted > plain, "boosted {boosted} vs plain {plain}");
}

#[test]
fn score_topic_artist_hint_boosts_confidence() {
    let q = query("Husn", "Anuv Jain", Some(180_000));
    let plain = accept_confidence(score(&q, "Husn", "", Some(180_000)));
    let topic = accept_confidence(score(&q, "Husn", "Anuv Jain Topic", Some(180_000)));
    assert!(topic > plain, "topic {topic} vs plain {plain}");
}

#[test]
fn score_rejects_when_title_tokens_do_not_overlap() {
    let q = query("Apna Bana Le", "Arijit Singh", None);
    let c = score(&q, "Blinding Lights", "The Weeknd", None);
    assert_eq!(reject_reason(c), RejectReason::NoTitleMatch);
}

#[test]
fn score_accepts_noisy_official_video_title() {
    let q = query("Apna Bana Le", "Arijit Singh", Some(261_000));
    let c = score(
        &q,
        "Apna Bana Le (Official Lyric Video) Varun Dhawan, Kriti S",
        "Sachin-J, Arijit S, Amitabh B",
        Some(261_360),
    );
    assert!(matches!(c, ScoreOutcome::Accept { .. }), "got {c:?}");
}

// ---------- LocalFile resolver ----------

#[test]
fn local_resolver_finds_library_track_and_reports_health() {
    let conn = open_memory().unwrap();
    let db = Arc::new(Mutex::new(conn));
    {
        let conn = db.lock().unwrap();
        track_insert(
            &conn,
            &NewTrack {
                source: "local".into(),
                path: Some("C:/music/apna_bana_le.mp3".into()),
                url: None,
                artist: "Arijit Singh".into(),
                title: "Apna Bana Le".into(),
                album: None,
                duration_ms: Some(261_360),
                year: None,
                track_no: None,
                disc_no: None,
                file_size: None,
                mtime: None,
            },
        )
        .unwrap();
    }
    let resolver = LocalFileResolver::new(db);

    let hit = resolver
        .resolve(&query("Apna Bana Le", "Arijit Singh", Some(261_000)))
        .now_or_never()
        .unwrap()
        .unwrap();
    assert_eq!(hit.url_or_path, "C:/music/apna_bana_le.mp3");
    assert_eq!(hit.source, "local");
    assert_eq!(hit.mime.as_deref(), Some("audio/mpeg"));
    assert!(hit.confidence >= 0.99);

    let miss = resolver
        .resolve(&query("Blinding Lights", "The Weeknd", None))
        .now_or_never()
        .unwrap();
    assert!(matches!(miss, Err(ResolveError::NotFound)));

    assert_eq!(
        resolver.health().now_or_never().unwrap().status,
        HealthStatus::Ok
    );
}
