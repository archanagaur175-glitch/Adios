//! P5.4 tests: enrichment matching. Required plan cases: exact title, case/
//! punctuation-insensitive, ±3 s duration window, artist-credit
//! disambiguation, no-match leaves the row untouched (Ok(None) → the
//! command writes nothing), release pick, fielded query, DB projection.
//! (End-to-end mock-transport cases live in `enrich_match_tests`.)

use crate::db::{self, queries, track_mb};
use crate::meta::enrich::*;
use crate::meta::mb_types::{MbArtistRef, MbCredit, MbRecording, MbReleaseRef};

fn track(title: &str, artist: &str, dur: Option<i64>) -> TrackMeta {
    TrackMeta {
        title: title.to_owned(),
        artist: artist.to_owned(),
        duration_ms: dur,
    }
}

fn recording(id: &str, title: &str, artist: &str, length: i64) -> MbRecording {
    MbRecording {
        id: id.to_owned(),
        score: Some(100),
        title: title.to_owned(),
        length: Some(length),
        artist_credit: Some(vec![MbCredit {
            artist: Some(MbArtistRef {
                id: None,
                name: artist.to_owned(),
                sort_name: None,
            }),
        }]),
        first_release_date: None,
        releases: None,
    }
}

fn stub_release(id: &str, status: &str, date: Option<&str>) -> MbReleaseRef {
    MbReleaseRef {
        id: id.to_owned(),
        status: Some(status.to_owned()),
        date: date.map(str::to_owned),
    }
}

#[test]
fn exact_title_and_duration_wins() {
    let t = track("back to friends", "sombr", Some(199_079));
    let cands = vec![
        recording("r-wrong-len", "back to friends", "sombr", 144_000),
        recording("r-hit", "back to friends", "sombr", 199_032),
    ];
    let hit = pick_match(&t, &cands).expect("exact title + Î”47 ms must match");
    assert_eq!(hit.id, "r-hit", "duration gate must drop the 144 s cut");
}

#[test]
fn title_match_is_case_and_punctuation_insensitive() {
    let t = track("Don't Stop Me Now!", "Queen", Some(209_000));
    let cands = vec![recording("r1", "dont stop me now", "Queen", 210_000)];
    assert!(
        pick_match(&t, &cands).is_some(),
        "apostrophe/case/! must not matter"
    );
    assert!((title_similarity("A.B", "a b") - 1.0).abs() < 1e-9);
    assert!(title_similarity("wholly different", "creep") < TITLE_MIN);
}

#[test]
fn duration_window_is_inclusive_at_three_seconds() {
    let inside = recording("ok", "Creep", "Radiohead", 200_000);
    let outside = recording("no", "Creep", "Radiohead", 196_999);
    let t = track("Creep", "Radiohead", Some(200_000));
    assert!(score_candidate(&t, &inside).is_some(), "Î”0 accepted");
    let t2 = track("Creep", "Radiohead", Some(197_001));
    assert!(score_candidate(&t2, &inside).is_some(), "Î”2999 accepted");
    assert!(
        score_candidate(&t, &outside).is_none(),
        "Î”3001 must reject"
    );
}

#[test]
fn artist_credit_disambiguates_same_title() {
    let t = track("Intro", "BjÃ¶rk", Some(100_000));
    let cands = vec![
        recording("r-other", "Intro", "The xx", 100_000),
        recording("r-mine", "Intro", "BjÃ¶rk", 100_000),
    ];
    let hit = pick_match(&t, &cands).expect("both title+duration fine â†’ accept");
    assert_eq!(hit.id, "r-mine", "credit must select BjÃ¶rk over The xx");
}

#[test]
fn mismatched_artist_is_rejected() {
    let t = track("Baby Doll", "Unknown Artist", Some(99_600));
    let cands = vec![recording("r1", "Baby Doll", "Someone Else", 99_600)];
    assert!(
        pick_match(&t, &cands).is_none(),
        "unknown local artist vs named credit = no signal match"
    );
}

#[test]
fn no_match_returns_none_row_would_be_untouched() {
    let t = track("Apna Bana Le - Bhediya (Official Lyrics Video) Varun Dhawan, Kriti S",
        "Lite Lyrics Official", Some(261_407));
    let cands = vec![recording("r1", "Apna Bana Le", "Arijit Singh", 271_000)];
    assert!(
        pick_match(&t, &cands).is_none(),
        "long rip title â‰  official title"
    );
    assert!(pick_match(&t, &[]).is_none(), "empty candidate set");
}

#[test]
fn pick_release_prefers_official_then_earliest_date() {
    let releases = vec![
        stub_release("bootleg", "Bootleg", Some("2024-01-01")),
        stub_release("official-late", "Official", Some("2025-06-01")),
        stub_release("official-early", "Official", Some("2024-11-30")),
        stub_release("no-date", "Official", None),
    ];
    let pick = pick_release(&releases).expect("always Some here");
    assert_eq!(pick.id, "official-early");
    assert_eq!(
        pick_release(&[stub_release("x", "Bootleg", None)]).map(|r| r.id.as_str()),
        Some("x"),
        "bootleg is still usable as a fallback"
    );
    assert!(pick_release(&[]).is_none());
}

#[test]
fn enrich_query_is_fielded_and_quote_safe() {
    assert_eq!(
        enrich_query("Creep", "Radiohead"),
        "recording:\"Creep\" AND artist:\"Radiohead\""
    );
    assert_eq!(enrich_query("Say \"What\"", ""), "recording:\"Say  What \"");
    assert_eq!(enrich_query("X", "   "), "recording:\"X\"");
}

#[test]
fn track_mb_roundtrip_projection() {
    let conn = db::open_memory().unwrap();
    let id = queries::track_insert(&conn, &queries::NewTrack {
        source: "local".into(),
        path: Some("C:\\x\\a.mp3".into()),
        url: None,
        artist: "sombr".into(),
        title: "back to friends".into(),
        album: None,
        duration_ms: Some(199_079),
        year: None,
        track_no: None,
        disc_no: None,
        file_size: None,
        mtime: None,
    }).unwrap();
    let rows = track_mb::unmatched(&conn, 50).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].duration_ms, Some(199_079));

    track_mb::set_mb_ids(&conn, id, "rec-1", Some("rel-1")).unwrap();
    assert!(track_mb::unmatched(&conn, 50).unwrap().is_empty());

    // A later match with no release must keep the stored release MBID.
    track_mb::set_mb_ids(&conn, id, "rec-2", None).unwrap();
    let (rec, rel): (String, String) = conn
        .query_row(
            "SELECT mb_recording_id, mb_release_id FROM tracks WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((rec.as_str(), rel.as_str()), ("rec-2", "rel-1"));
}

