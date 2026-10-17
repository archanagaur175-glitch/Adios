//! P16 tests: the genre column (migration 004 + scan backfill semantics).

use super::queries::{self, NewTrack};
use super::*;

#[test]
fn genre_column_migrates_and_backfills_only_nulls() {
    let conn = open_memory().expect("migrate");
    let new = NewTrack {
        source: "local".into(),
        path: Some("C:/music/a.mp3".into()),
        url: None,
        artist: "Boards of Canada".into(),
        title: "Roygbiv".into(),
        album: None,
        duration_ms: Some(151_000),
        year: Some(1998),
        track_no: None,
        disc_no: None,
        file_size: None,
        mtime: None,
    };
    let id = queries::track_insert(&conn, &new).expect("insert");
    let fresh = queries::track_get(&conn, id).expect("get").expect("row");
    assert_eq!(fresh.genre, None, "new row starts with no genre");

    // Rescan backfills the empty column…
    queries::track_set_genre(&conn, "C:/music/a.mp3", "Electronic").expect("set");
    // …but never clobbers a genre the row already has.
    queries::track_set_genre(&conn, "C:/music/a.mp3", "Ambient").expect("set again");
    let after = queries::track_get(&conn, id).expect("get").expect("row");
    assert_eq!(after.genre.as_deref(), Some("Electronic"));
}

#[test]
fn genre_flows_through_playlist_and_history_reads() {
    let conn = open_memory().expect("migrate");
    let new = NewTrack {
        source: "local".into(),
        path: Some("C:/music/b.mp3".into()),
        url: None,
        artist: "Aphex Twin".into(),
        title: "Xtal".into(),
        album: None,
        duration_ms: None,
        year: None,
        track_no: None,
        disc_no: None,
        file_size: None,
        mtime: None,
    };
    let id = queries::track_insert(&conn, &new).expect("insert");
    queries::track_set_genre(&conn, "C:/music/b.mp3", "Electronic").expect("set");
    let pl = super::playlist::create(&conn, "Taste").expect("playlist");
    super::playlist::add_track(&conn, pl, id).expect("add");

    let detail = super::playlist::detail(&conn, pl).expect("detail").expect("some");
    assert_eq!(
        detail.tracks[0].genre.as_deref(),
        Some("Electronic"),
        "TRACK_COLUMNS replacement keeps genre in playlist reads"
    );
}
