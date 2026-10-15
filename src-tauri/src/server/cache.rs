//! Disk-cache bookkeeping: `cache_index` rows + deterministic cache paths.
//! Write path is driven by `super::proxy` (tee → temp→rename → index here).

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

/// Row from `cache_index` for a previously cached URL.
pub struct CacheRow {
    pub local_path: String,
    pub bytes: i64,
}

pub fn cache_index_get(conn: &Connection, url: &str) -> Option<CacheRow> {
    conn.query_row(
        "SELECT local_path, bytes FROM cache_index WHERE url = ?1",
        [url],
        |row| {
            Ok(CacheRow {
                local_path: row.get(0)?,
                bytes: row.get(1)?,
            })
        },
    )
    .optional()
    .ok()
    .flatten()
}

/// Record a completed cache download (upsert on url).
pub fn cache_index_put(
    conn: &Connection,
    url: &str,
    local_path: &str,
    mime: Option<&str>,
    bytes: u64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO cache_index (url, local_path, mime, bytes, last_access, hit_count)
         VALUES (?1, ?2, ?3, ?4, datetime('now'), 0)
         ON CONFLICT(url) DO UPDATE SET
           local_path = excluded.local_path, mime = excluded.mime,
           bytes = excluded.bytes, last_access = datetime('now')",
        params![url, local_path, mime, bytes as i64],
    )?;
    Ok(())
}

/// Bump `hit_count` + `last_access` after serving from cache.
pub fn cache_index_touch(conn: &Connection, url: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE cache_index SET hit_count = hit_count + 1, last_access = datetime('now')
         WHERE url = ?1",
        [url],
    )?;
    Ok(())
}

fn ext_for_mime(mime: &str) -> &'static str {
    match mime {
        "audio/mp4" => "m4a",
        "audio/webm" => "webm",
        "audio/mpeg" => "mp3",
        _ => "bin",
    }
}

/// Deterministic final cache path for a URL (FNV-1a of the URL + mime ext).
pub fn cache_path_for_url(cache_dir: &Path, url: &str, mime: &str) -> PathBuf {
    cache_dir.join(format!(
        "{}.{}",
        crate::resolver::fnv1a_hex(url),
        ext_for_mime(mime)
    ))
}
