use super::*;
use crate::db::queries::{self, NewTrack};

fn insert_track(conn: &Connection, artist: &str, title: &str, album: &str) -> i64 {
    conn.execute(
        "INSERT INTO tracks (source, artist, title, album) VALUES ('local', ?1, ?2, ?3)",
        rusqlite::params![artist, title, album],
    )
    .expect("insert track");
    conn.last_insert_rowid()
}

fn fts_matches(conn: &Connection, query: &str) -> i64 {
    let mut stmt = conn
        .prepare("SELECT count(*) FROM tracks_fts WHERE tracks_fts MATCH ?1")
        .expect("prepare fts");
    stmt.query_row([query], |row| row.get(0)).unwrap_or(0)
}

#[test]
fn fresh_migration_creates_all_tables() {
    let conn = open_memory().expect("open+migrate");
    for table in [
        "tracks",
        "tracks_fts",
        "playlists",
        "playlist_tracks",
        "queue",
        "cache_index",
        "lyrics_cache",
        "meta_cache",
        "settings",
        "schema_migrations",
    ] {
        let name: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type IN ('table','view') AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| panic!("missing table: {table}"));
        assert_eq!(name, table);
    }
}

#[test]
fn migration_rerun_is_noop() {
    let conn = open_memory().expect("first migrate");
    let first = migrate(&conn).expect("re-run migrate");
    assert!(first.is_empty(), "second run must apply nothing");
    let count: i64 = conn
        .query_row("SELECT count(*) FROM schema_migrations", [], |r| r.get(0))
        .expect("count");
    assert_eq!(count, crate::db::migrations::MIGRATIONS.len() as i64);
}

#[test]
fn version_mismatch_skips_reapplied_migration() {
    let conn = Connection::open_in_memory().expect("open");
    conn.execute_batch(SCHEMA_VERSION_TABLE).unwrap();
    // Fake EVERY migration as applied without its tables — this test is
    // about version-gating, not SQL validity (002 ALTERs a table 001 made).
    for (version, name) in [
        (1i64, "001_init"),
        (2i64, "002_lyrics_cache"),
        (3i64, "003_download_pins"),
        (4i64, "004_genre"),
    ] {
        conn.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![version, name],
        )
        .unwrap();
    }
    let applied = migrate(&conn).expect("migrate");
    assert!(applied.is_empty());
    let exists = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'tracks'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();
    assert_eq!(exists, 0, "already-versioned migration must not run");
}

#[test]
fn fts_triggers_track_insert_update_delete() {
    let conn = open_memory().expect("migrate");
    let id = insert_track(&conn, "Massive Attack", "Teardrop", "Mezzanine");

    assert_eq!(fts_matches(&conn, "Teardrop"), 1, "insert synced into fts");
    assert_eq!(fts_matches(&conn, "teardrop"), 1, "case-insensitive match");

    conn.execute("UPDATE tracks SET title = 'Angel' WHERE id = ?1", [id]).expect("update");
    assert_eq!(fts_matches(&conn, "Teardrop"), 0, "old value removed");
    assert_eq!(fts_matches(&conn, "Angel"), 1, "new value indexed");

    conn.execute("DELETE FROM tracks WHERE id = ?1", [id])
        .expect("delete");
    assert_eq!(fts_matches(&conn, "Angel"), 0, "delete removes fts row");
}

#[test]
fn track_crud_roundtrip() {
    let conn = open_memory().expect("migrate");
    let new = NewTrack {
        source: "local".into(),
        path: Some("C:/music/a.mp3".into()),
        url: None,
        artist: "Boards of Canada".into(),
        title: "Roygbiv".into(),
        album: Some("Music Has the Right to Children".into()),
        duration_ms: Some(151_000),
        year: Some(1998),
        track_no: Some(2),
        disc_no: Some(1),
        file_size: Some(2_400_000),
        mtime: Some(1_700_000_000),
    };
    let id = queries::track_insert(&conn, &new).expect("insert");

    let fetched = queries::track_get(&conn, id).expect("get").expect("some");
    assert_eq!(fetched.title, "Roygbiv");
    assert_eq!(fetched.artist, "Boards of Canada");
    assert_eq!(fetched.duration_ms, Some(151_000));
    assert_eq!(fetched.source, "local");

    // Duplicate path must not create a second row; returns the existing id.
    let dup_id = queries::track_insert(&conn, &new).expect("dup insert");
    assert_eq!(dup_id, id);
    let all = queries::tracks_all(&conn).expect("all");
    assert_eq!(all.len(), 1);

    let deleted = queries::track_delete(&conn, id).expect("delete");
    assert_eq!(deleted, 1);
    assert!(queries::track_get(&conn, id).expect("get after delete").is_none());
}

#[test]
fn tracks_all_orders_by_artist_then_title() {
    let conn = open_memory().expect("migrate");
    for (artist, title) in [
        ("Aphex Twin", "Xtal"),
        ("Aphex Twin", "Tha"),
        ("Boards of Canada", "Roygbiv"),
    ] {
        let track = NewTrack {
            source: "local".into(),
            path: Some(format!("/{artist}/{title}.mp3")),
            url: None,
            artist: artist.into(),
            title: title.into(),
            album: None,
            duration_ms: None,
            year: None,
            track_no: None,
            disc_no: None,
            file_size: None,
            mtime: None,
        };
        queries::track_insert(&conn, &track).expect("insert");
    }
    let titles: Vec<String> = queries::tracks_all(&conn)
        .expect("all")
        .into_iter()
        .map(|t| t.title)
        .collect();
    // Same artist sorts by title (track_no is NULL): "Tha" < "Xtal" (NOCASE).
    assert_eq!(titles, vec!["Tha", "Xtal", "Roygbiv"]);
}

#[test]
fn settings_roundtrip_and_upsert() {
    let conn = open_memory().expect("migrate");
    queries::settings_set(&conn, "crossfadeMs", "0").expect("set");
    queries::settings_set(&conn, "contactEmail", "\"user@example.com\"").expect("set");
    let all = queries::settings_get_all(&conn).expect("get_all");
    assert_eq!(all.get("crossfadeMs").map(String::as_str), Some("0"));
    assert_eq!(
        all.get("contactEmail").map(String::as_str),
        Some("\"user@example.com\"")
    );

    queries::settings_set(&conn, "crossfadeMs", "250").expect("upsert");
    let all = queries::settings_get_all(&conn).expect("get_all again");
    assert_eq!(all.get("crossfadeMs").map(String::as_str), Some("250"));
    assert_eq!(all.len(), 2, "upsert must not add a row");
}

#[test]
fn fts_search_matches_title_and_artist() {
    let conn = open_memory().expect("migrate");
    insert_track(&conn, "Portishead", "Roads", "Dummy");
    insert_track(&conn, "Bonobo", "Kerala", "Migration");
    assert_eq!(fts_matches(&conn, "Roads"), 1);
    assert_eq!(fts_matches(&conn, "portishead"), 1);
    assert_eq!(fts_matches(&conn, "Kerala OR Roads"), 2);
    assert_eq!(fts_matches(&conn, "nonexistent"), 0);
}
