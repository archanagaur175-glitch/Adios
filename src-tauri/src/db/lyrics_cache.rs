//! lyrics_cache access (P7.1): hit rows carry the LRCLIB strings; negative
//! rows (status='not_found') exist so a 404 / instrumental answer never
//! re-hits the network for the same normalized query key.
//! Note: m001 named the synced column `synced_json` — it stores the LRC
//! text exactly as LRCLIB returns it (not JSON).

use rusqlite::{params, Connection};

use crate::meta::lrclib_types::LyricsPayload;
use crate::resolver::SharedDb;

const SELECT: &str = "SELECT status, synced_json, plain_text, instrumental, lrclib_id \
                      FROM lyrics_cache WHERE track_key = ?1";

pub fn get(db: &SharedDb, key: &str) -> Result<Option<LyricsPayload>, String> {
    let conn = db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
    get_conn(&conn, key)
}

pub fn get_conn(conn: &Connection, key: &str) -> Result<Option<LyricsPayload>, String> {
    let mut stmt = conn
        .prepare(SELECT)
        .map_err(|e| format!("lyrics cache read: {e}"))?;
    let mut rows = stmt
        .query_map(params![key], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, Option<i64>>(4)?,
            ))
        })
        .map_err(|e| format!("lyrics cache read: {e}"))?;
    let Some(row) = rows.next() else {
        return Ok(None);
    };
    let (status, synced, plain, instrumental, lrclib_id) =
        row.map_err(|e| format!("lyrics cache read: {e}"))?;
    Ok(Some(LyricsPayload {
        found: status == "hit",
        synced,
        plain,
        instrumental: instrumental != 0,
        lrclib_id,
        source: "cache".to_owned(),
    }))
}

/// Upsert a found row (status='hit') from a network payload.
pub fn put_hit(db: &SharedDb, key: &str, p: &LyricsPayload) -> Result<(), String> {
    upsert(
        db,
        key,
        "hit",
        p.synced.as_deref(),
        p.plain.as_deref(),
        p.instrumental,
        p.lrclib_id,
    )
}

/// Upsert a negative row (404 or 200-without-lyrics; `instrumental` keeps
/// the reason for provenance).
pub fn put_not_found(db: &SharedDb, key: &str, instrumental: bool) -> Result<(), String> {
    upsert(db, key, "not_found", None, None, instrumental, None)
}

#[allow(clippy::too_many_arguments)]
fn upsert(
    db: &SharedDb,
    key: &str,
    status: &str,
    synced: Option<&str>,
    plain: Option<&str>,
    instrumental: bool,
    lrclib_id: Option<i64>,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| format!("db lock poisoned: {e}"))?;
    conn.execute(
        "INSERT INTO lyrics_cache
           (track_key, status, synced_json, plain_text, instrumental, lrclib_id, fetched_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
         ON CONFLICT(track_key) DO UPDATE SET
           status = excluded.status,
           synced_json = excluded.synced_json,
           plain_text = excluded.plain_text,
           instrumental = excluded.instrumental,
           lrclib_id = excluded.lrclib_id,
           fetched_at = excluded.fetched_at",
        params![key, status, synced, plain, instrumental as i64, lrclib_id],
    )
    .map_err(|e| format!("lyrics cache write: {e}"))?;
    Ok(())
}
