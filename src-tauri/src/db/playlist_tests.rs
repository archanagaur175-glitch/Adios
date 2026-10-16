//! P8.2 playlist tests: CRUD, dense positions, reorder validation, and the
//! versioned export/import roundtrip by stable track identity.

use rusqlite::Connection;

use super::playlist::{add_track, create, delete, detail, list, remove_track, rename, reorder};
use super::playlist_io::{export, import, PlaylistExport, EXPORT_VERSION};
use super::queries::{track_insert, NewTrack};

fn conn() -> Connection {
    crate::db::open_memory().unwrap()
}

/// Insert a minimal local track; `mb` optionally sets an MB recording id.
fn seed_track(conn: &Connection, title: &str, path: &str, mb: Option<&str>) -> i64 {
    let id = track_insert(
        conn,
        &NewTrack {
            source: "local".into(),
            path: Some(path.into()),
            url: None,
            artist: "Seed".into(),
            title: title.into(),
            album: None,
            duration_ms: Some(180_000),
            year: None,
            track_no: None,
            disc_no: None,
            file_size: None,
            mtime: None,
        },
    )
    .unwrap();
    if let Some(mb) = mb {
        conn.execute("UPDATE tracks SET mb_recording_id = ?1 WHERE id = ?2", rusqlite::params![mb, id])
            .unwrap();
    }
    id
}

fn member_ids(conn: &Connection, playlist_id: i64) -> Vec<i64> {
    let d = detail(conn, playlist_id).unwrap().unwrap();
    d.tracks.into_iter().map(|t| t.id).collect()
}

#[test]
fn create_rename_delete_roundtrip() {
    let c = conn();
    let id = create(&c, "Road trip").unwrap();
    let all = list(&c).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].name.as_str(), all[0].track_count), ("Road trip", 0));

    assert!(rename(&c, id, "Road trip v2").unwrap());
    assert_eq!(list(&c).unwrap()[0].name, "Road trip v2");
    assert!(delete(&c, id).unwrap());
    assert!(list(&c).unwrap().is_empty());
    assert!(!delete(&c, id).unwrap()); // gone
    assert!(detail(&c, id).unwrap().is_none());
}

#[test]
fn add_remove_keeps_dense_positions() {
    let c = conn();
    let a = seed_track(&c, "A", "C:/m/a.mp3", None);
    let b = seed_track(&c, "B", "C:/m/b.mp3", None);
    let p = create(&c, "Mix").unwrap();

    assert!(add_track(&c, p, a).unwrap());
    assert!(add_track(&c, p, b).unwrap());
    assert!(!add_track(&c, p, a).unwrap()); // duplicate → false, no dupe row
    assert_eq!(member_ids(&c, p), vec![a, b]);

    assert!(remove_track(&c, p, a).unwrap());
    assert_eq!(member_ids(&c, p), vec![b]);
    let pos: i64 = c
        .query_row(
            "SELECT position FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
            rusqlite::params![p, b],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(pos, 0, "surviving member renumbered densely");
    assert!(!remove_track(&c, p, a).unwrap()); // already gone
    assert!(!add_track(&c, 999, b).unwrap()); // missing playlist → false
}

#[test]
fn reorder_rewrites_positions_and_rejects_foreign_sets() {
    let c = conn();
    let a = seed_track(&c, "A", "C:/m/a.mp3", None);
    let b = seed_track(&c, "B", "C:/m/b.mp3", None);
    let d = seed_track(&c, "D", "C:/m/d.mp3", None);
    let p = create(&c, "Order").unwrap();
    for id in [a, b, d] {
        add_track(&c, p, id).unwrap();
    }

    assert!(reorder(&c, p, &[d, a, b]).unwrap());
    assert_eq!(member_ids(&c, p), vec![d, a, b]);

    // Wrong membership (dropped + foreign id) must not half-apply.
    assert!(!reorder(&c, p, &[a, b]).unwrap());
    assert_eq!(member_ids(&c, p), vec![d, a, b]);
}

#[test]
fn delete_cascades_members_but_not_tracks() {
    let c = conn();
    let a = seed_track(&c, "A", "C:/m/a.mp3", None);
    let p = create(&c, "Gone").unwrap();
    add_track(&c, p, a).unwrap();
    delete(&c, p).unwrap();
    let members: i64 = c
        .query_row("SELECT COUNT(*) FROM playlist_tracks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(members, 0, "FK cascade removed members");
    let tracks: i64 = c
        .query_row("SELECT COUNT(*) FROM tracks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(tracks, 1, "library rows untouched");
}

#[test]
fn export_import_roundtrip_by_stable_identity() {
    let c = conn();
    let t_path = seed_track(&c, "Pathy", "C:/m/pathy.mp3", None);
    let t_mb = seed_track(&c, "Mby", "C:/m/mby.mp3", Some("mbid-123"));
    let t_url = seed_track(&c, "Urly", "C:/m/urly.mp3", None);
    c.execute("UPDATE tracks SET url = 'https://x.invalid/s' WHERE id = ?1", [t_url])
        .unwrap();
    let p = create(&c, "Favs").unwrap();
    for id in [t_path, t_mb, t_url] {
        add_track(&c, p, id).unwrap();
    }

    let dump = export(&c, p).unwrap().unwrap();
    assert_eq!(dump.version, EXPORT_VERSION);
    // Schema check: the export survives a JSON serialize/deserialize round.
    let text = serde_json::to_string_pretty(&dump).unwrap();
    let parsed: PlaylistExport = serde_json::from_str(&text).unwrap();

    // A machine whose library lacks the middle track: it counts as missing.
    let c2 = conn();
    let t2 = seed_track(&c2, "Pathy", "C:/m/pathy.mp3", None);
    let t_urly = seed_track(&c2, "Urly", "C:/m/urly.mp3", None);
    c2.execute("UPDATE tracks SET url = 'https://x.invalid/s' WHERE id = ?1", [t_urly])
        .unwrap();

    let result = import(&c2, &parsed).unwrap();
    assert_eq!((result.added, result.missing), (2, 1));
    // Resolved members keep the EXPORT order of the survivors.
    assert_eq!(member_ids(&c2, result.playlist_id), vec![t2, t_urly]);
}

#[test]
fn import_missing_playlist_export_is_none_and_version_is_declared() {
    let c = conn();
    assert!(export(&c, 42).unwrap().is_none());
    assert_eq!(EXPORT_VERSION, 1);
}
