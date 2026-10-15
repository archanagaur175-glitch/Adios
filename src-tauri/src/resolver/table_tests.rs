//! Resolution-table semantics: the "exactly one auto re-resolve" guarantee.

use super::table::ResolutionTable;
use super::Query;

fn query() -> Query {
    Query {
        artist: "Arijit Singh".to_owned(),
        title: "Apna Bana Le".to_owned(),
        album: None,
        duration_ms: Some(261_000),
    }
}

#[test]
fn query_for_returns_registered_query() {
    let table = ResolutionTable::new();
    assert!(table.query_for("https://x/a").is_none());
    table.register("https://x/a", &query());
    let q = table.query_for("https://x/a").expect("entry");
    assert_eq!(q.title, "Apna Bana Le");
}

#[test]
fn claim_succeeds_once_then_denied() {
    let table = ResolutionTable::new();
    table.register("https://x/a", &query());
    assert!(table.claim("https://x/a"), "first claim must win");
    assert!(!table.claim("https://x/a"), "second claim must be denied");
    assert!(!table.claim("https://x/unknown"), "unknown urls never claim");
}

#[test]
fn retarget_carries_attempt_count_forward() {
    let table = ResolutionTable::new();
    table.register("https://x/a", &query());
    assert!(table.claim("https://x/a"));
    table.retarget("https://x/a", "https://x/b", &query());

    assert!(table.query_for("https://x/a").is_none(), "entry moved");
    assert!(table.query_for("https://x/b").is_some());
    assert!(
        !table.claim("https://x/b"),
        "fresh url from a claimed re-resolve gets no second attempt"
    );
}

#[test]
fn register_resets_attempts_for_new_lifecycle() {
    let table = ResolutionTable::new();
    table.register("https://x/a", &query());
    assert!(table.claim("https://x/a"));
    assert!(!table.claim("https://x/a"));
    table.register("https://x/a", &query());
    assert!(
        table.claim("https://x/a"),
        "an explicit resolve re-arms the single attempt"
    );
}
