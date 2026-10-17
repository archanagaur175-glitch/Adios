//! P9.7 tests: stream track persistence (dedupe, url refresh, row shape).

use super::stream::stream_track_upsert;

fn upsert(
    conn: &rusqlite::Connection,
    url: &str,
    mb: Option<&str>,
) -> crate::db::queries::TrackRow {
    stream_track_upsert(conn, url, "Rick Astley", "Never Gonna Give You Up", Some(213_000), mb, None)
        .expect("upsert")
}

#[test]
fn inserts_stream_row_with_positive_db_id() {
    let conn = crate::db::open_memory().expect("db");
    let row = upsert(&conn, "https://cdn/x.m4a", Some("dz:1"));
    assert!(row.id > 0, "DB rows get positive ids");
    assert_eq!(row.source, "stream");
    assert_eq!(row.path, None);
    assert_eq!(row.url.as_deref(), Some("https://cdn/x.m4a"));
    assert_eq!(row.mb_recording_id.as_deref(), Some("dz:1"));
    assert_eq!(row.play_count, 0);
}

#[test]
fn dedupes_by_mb_recording_id_and_refreshes_url() {
    let conn = crate::db::open_memory().expect("db");
    let a = upsert(&conn, "https://cdn/old.m4a", Some("dz:1"));
    let b = upsert(&conn, "https://cdn/fresh.m4a", Some("dz:1"));
    assert_eq!(a.id, b.id, "same identity must reuse the row");
    assert_eq!(b.url.as_deref(), Some("https://cdn/fresh.m4a"));
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))
        .expect("count");
    assert_eq!(count, 1);
}

#[test]
fn distinct_ids_create_distinct_rows() {
    let conn = crate::db::open_memory().expect("db");
    let a = upsert(&conn, "https://cdn/1.m4a", Some("dz:1"));
    let b = upsert(&conn, "https://cdn/2.m4a", Some("dz:2"));
    assert_ne!(a.id, b.id);
}

#[test]
fn stream_track_is_fts_searchable() {
    let conn = crate::db::open_memory().expect("db");
    upsert(&conn, "https://cdn/x.m4a", Some("dz:9"));
    let hits = crate::db::fts::search(&conn, "Never Gonna", 10).expect("fts");
    assert!(
        hits.iter().any(|t| t.source == "stream"),
        "insert triggers must index stream rows"
    );
}

#[test]
fn genre_is_stored_and_kept_on_repick() {
    let conn = crate::db::open_memory().expect("db");
    let row = stream_track_upsert(
        &conn,
        "https://cdn/x.m4a",
        "Rick Astley",
        "Never Gonna Give You Up",
        Some(213_000),
        Some("itunes:99"),
        Some("Pop"),
    )
    .expect("insert with genre");
    assert_eq!(row.genre.as_deref(), Some("Pop"));

    // Re-pick without a genre must not erase what the row already knows.
    let refreshed = stream_track_upsert(
        &conn,
        "https://cdn/fresh.m4a",
        "Rick Astley",
        "Never Gonna Give You Up",
        Some(213_000),
        Some("itunes:99"),
        None,
    )
    .expect("re-pick");
    assert_eq!(refreshed.id, row.id);
    assert_eq!(refreshed.genre.as_deref(), Some("Pop"), "COALESCE keeps genre");
}
