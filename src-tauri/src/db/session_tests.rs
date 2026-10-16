//! P8.3 session tests: queue save/load roundtrip, empty state, cascade
//! reindex (deleted tracks) with dropped count, and the cached-track view.

use rusqlite::Connection;

use super::queries::{track_insert, NewTrack};
use super::session::{cached_tracks, load_queue, save_queue};

fn conn() -> Connection {
    crate::db::open_memory().unwrap()
}

fn seed_remote(conn: &Connection, title: &str, url: Option<&str>) -> i64 {
    track_insert(
        conn,
        &NewTrack {
            source: if url.is_some() { "remote" } else { "local" }.into(),
            path: None,
            url: url.map(str::to_string),
            artist: "Seed".into(),
            title: title.into(),
            album: None,
            duration_ms: Some(100_000),
            year: None,
            track_no: None,
            disc_no: None,
            file_size: None,
            mtime: None,
        },
    )
    .unwrap()
}

fn cache_row(conn: &Connection, url: &str, path: &str) {
    conn.execute(
        "INSERT INTO cache_index (url, local_path, bytes) VALUES (?1, ?2, 1)",
        rusqlite::params![url, path],
    )
    .unwrap();
}

#[test]
fn save_load_roundtrip_keeps_order_and_index() {
    let c = conn();
    let a = seed_remote(&c, "A", Some("https://x.invalid/a"));
    let b = seed_remote(&c, "B", Some("https://x.invalid/b"));
    save_queue(&c, &[a, b], 1).unwrap();
    let snap = load_queue(&c).unwrap();
    assert_eq!(snap.track_ids, vec![a, b]);
    assert_eq!(snap.queue_index, 1);
    assert_eq!(snap.dropped, 0);

    save_queue(&c, &[], -1).unwrap();
    let empty = load_queue(&c).unwrap();
    assert!(empty.track_ids.is_empty());
    assert_eq!(empty.queue_index, -1);
}

#[test]
fn index_is_clamped_on_save_and_load() {
    let c = conn();
    let a = seed_remote(&c, "A", Some("https://x.invalid/a"));
    save_queue(&c, &[a], 7).unwrap();
    assert_eq!(load_queue(&c).unwrap().queue_index, 0);
}

#[test]
fn deleted_track_cascades_and_load_reindexes() {
    let c = conn();
    let a = seed_remote(&c, "A", Some("https://x.invalid/a"));
    let b = seed_remote(&c, "B", Some("https://x.invalid/b"));
    let d = seed_remote(&c, "D", Some("https://x.invalid/d"));
    save_queue(&c, &[a, b, d], 2).unwrap();

    // Deleting the middle track cascades its queue row (FK) and leaves a
    // sparse position — load must densify and report dropped=1.
    c.execute("DELETE FROM tracks WHERE id = ?1", [b]).unwrap();
    let snap = load_queue(&c).unwrap();
    assert_eq!(snap.track_ids, vec![a, d]);
    assert_eq!(snap.dropped, 1);
    assert_eq!(snap.queue_index, 1, "index 2 clamped to last after drop");

    // Self-healed rows are dense in the table itself.
    let positions: Vec<i64> = {
        let mut stmt = c
            .prepare("SELECT position FROM queue ORDER BY position")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    };
    assert_eq!(positions, vec![0, 1]);
}

#[test]
fn cached_tracks_lists_only_remote_rows_with_cache_entries() {
    let c = conn();
    let cached = seed_remote(&c, "Cached", Some("https://x.invalid/s"));
    seed_remote(&c, "Uncached", Some("https://x.invalid/u"));
    seed_remote(&c, "LocalOnly", None);
    cache_row(&c, "https://x.invalid/s", "C:/cache/s.bin");

    let rows = cached_tracks(&c).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, cached);
    assert_eq!(rows[0].title, "Cached");
}
