//! P5.3 FTS tests: prefix matching across fields, operator-safe patterns,
//! limits, and empty input.

use rusqlite::Connection;

use super::fts;
use crate::db::open_memory;
use crate::db::queries::{self, NewTrack};

fn insert(conn: &Connection, artist: &str, title: &str, album: &str) {
    let track = NewTrack {
        source: "local".into(),
        path: Some(format!("/{artist}/{title}.mp3")),
        url: None,
        artist: artist.into(),
        title: title.into(),
        album: Some(album.into()),
        duration_ms: None,
        year: None,
        track_no: None,
        disc_no: None,
        file_size: None,
        mtime: None,
    };
    queries::track_insert(conn, &track).expect("insert");
}

fn fixture() -> Connection {
    let conn = open_memory().expect("migrate");
    insert(&conn, "Portishead", "Roads", "Dummy");
    insert(&conn, "Bonobo", "Kerala", "Migration");
    insert(&conn, "And One", "Body Music", "Body Music");
    conn
}

fn titles(conn: &Connection, query: &str, limit: i64) -> Vec<String> {
    fts::search(conn, query, limit)
        .expect("search")
        .into_iter()
        .map(|t| t.title)
        .collect()
}

#[test]
fn match_pattern_quotes_and_prefixes_each_token() {
    assert_eq!(fts::match_pattern("blinding lights"), Some("\"blinding\"* \"lights\"*".into()));
    assert_eq!(fts::match_pattern("  spaced   out  "), Some("\"spaced\"* \"out\"*".into()));
    assert_eq!(fts::match_pattern(""), None);
    assert_eq!(fts::match_pattern("   \t "), None);
    // FTS operators become literal text, quotes inside tokens are escaped.
    assert_eq!(fts::match_pattern("a AND b"), Some("\"a\"* \"AND\"* \"b\"*".into()));
    assert_eq!(fts::match_pattern("x\"y"), Some("\"x\"\"y\"*".into()));
}

#[test]
fn search_matches_title_artist_album_with_prefix() {
    let conn = fixture();
    assert_eq!(titles(&conn, "road", 10), vec!["Roads"], "title prefix");
    assert_eq!(titles(&conn, "portis", 10), vec!["Roads"], "artist prefix");
    assert_eq!(titles(&conn, "migra", 10), vec!["Kerala"], "album prefix");
    // Space between tokens = implicit AND in FTS5: both terms in ONE row.
    assert_eq!(titles(&conn, "portis dummy", 10), vec!["Roads"]);
    assert!(titles(&conn, "kerala dummy", 10).is_empty(), "AND across rows");
}

#[test]
fn operators_and_empty_input_never_error() {
    let conn = fixture();
    // "AND" as a user token must be literal (it matches "And One").
    assert_eq!(titles(&conn, "and", 10), vec!["Body Music"]);
    // Same for "OR": quoted as a literal term, so this yields nothing.
    assert!(titles(&conn, "road OR kerala", 10).is_empty(), "OR stays literal");
    assert!(titles(&conn, "weird:\"<>(", 10).is_empty(), "junk yields no rows");
    assert!(titles(&conn, "", 10).is_empty(), "empty query");
    assert!(titles(&conn, "   ", 10).is_empty(), "whitespace query");
}

#[test]
fn search_respects_limit() {
    let conn = fixture();
    let all = titles(&conn, "b", 10);
    assert_eq!(all.len(), 2, "prefix 'b' matches Bonobo + And One: {all:?}");
    assert_eq!(fts::search(&conn, "b", 1).expect("limited").len(), 1, "limit 1");
}
