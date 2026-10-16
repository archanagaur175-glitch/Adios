//! Play history (P10): the m001 `tracks.play_count` / `last_played_at`
//! columns — written on every track start, read back for the History tab.
//! One row per song: repeats bump the counter instead of adding rows.

use rusqlite::{params, Connection};

use super::queries::{row_to_track, TrackRow, TRACK_COLUMNS};

/// One play: bump the counter and stamp the clock (UTC via SQLite).
pub fn record_play(conn: &Connection, id: i64) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE tracks SET play_count = play_count + 1, last_played_at = datetime('now')
         WHERE id = ?1",
        params![id],
    )
}

/// Most-recently-played tracks (History tab); `limit` bounds the view.
pub fn play_history(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<TrackRow>> {
    let sql = format!(
        "SELECT {TRACK_COLUMNS} FROM tracks
         WHERE last_played_at IS NOT NULL
         ORDER BY last_played_at DESC, id DESC
         LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![limit], row_to_track)?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;
    use crate::db::queries::{track_insert, NewTrack};

    fn seed(conn: &Connection, title: &str) -> i64 {
        track_insert(
            conn,
            &NewTrack {
                source: "local".into(),
                path: Some(format!("/x/{title}")),
                url: None,
                artist: "A".into(),
                title: title.into(),
                album: None,
                duration_ms: None,
                year: None,
                track_no: None,
                disc_no: None,
                file_size: None,
                mtime: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn record_play_increments_and_stamps() {
        let conn = open_memory().unwrap();
        let id = seed(&conn, "one");
        assert_eq!(record_play(&conn, id).unwrap(), 1);
        assert_eq!(record_play(&conn, id).unwrap(), 1);
        let rows = play_history(&conn, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].play_count, 2);
        assert!(rows[0].last_played_at.is_some());
    }

    #[test]
    fn history_excludes_never_played_and_limits() {
        let conn = open_memory().unwrap();
        seed(&conn, "cold");
        let a = seed(&conn, "a");
        let b = seed(&conn, "b");
        record_play(&conn, a).unwrap();
        // Same-second stamps: id DESC breaks the tie so the later play wins.
        record_play(&conn, b).unwrap();
        let rows = play_history(&conn, 1).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, b);
        let all = play_history(&conn, 10).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|t| t.last_played_at.is_some()));
    }
}
