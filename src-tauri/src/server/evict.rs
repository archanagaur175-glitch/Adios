//! Disk-cache eviction (P8.1): enforces the `cacheSizeMb` cap with LRU
//! victim order and keeps `cache_index` consistent with the files on disk.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

/// Stats from one eviction pass (serde camelCase for the webview).
#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvictStats {
    pub removed: usize,         // dangling rows + beyond-cap LRU rows
    pub removed_parts: usize,   // stale `.part-*` (> 1 h = no live download)
    pub removed_orphans: usize, // unreferenced finals (> 5 min = crash leftovers)
    pub freed_bytes: u64,
    pub remaining_bytes: u64,
}

const DEFAULT_CAP_MB: u64 = 512;
/// Floor so one typo cannot wipe the cache (the FE input enforces the same 16).
const MIN_CAP_MB: u64 = 16;
const PART_MAX_AGE_SECS: u64 = 3600;
const ORPHAN_MIN_AGE_SECS: u64 = 300;

/// `cacheSizeMb` setting → bytes (default 512 MB, floor 16 MB).
pub fn cache_cap_bytes(conn: &Connection) -> u64 {
    let raw = conn
        .query_row(
            "SELECT value_json FROM settings WHERE key = 'cacheSizeMb'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten();
    let mb = raw
        .and_then(|s| serde_json::from_str::<u64>(&s).ok())
        .unwrap_or(DEFAULT_CAP_MB);
    mb.clamp(MIN_CAP_MB, 1_000_000) * 1024 * 1024
}

fn index_total(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(bytes), 0) FROM cache_index",
        [],
        |row| row.get(0),
    )
}

/// One pass: sweep temp/orphan leftovers, drop rows whose file vanished,
/// then delete least-recently-used rows (oldest `last_access`, then lowest
/// `hit_count`) until the index fits `cap_bytes`. Set-based — one pass always
/// lands at/under the cap; files still open in a serve survive as orphan
/// files and go on a later sweep. Leaves `cache_index` consistent: every
/// remaining row points at an existing file.
pub fn evict_to_cap(
    conn: &Connection,
    cache_dir: &Path,
    cap_bytes: u64,
) -> rusqlite::Result<EvictStats> {
    let mut stats = EvictStats::default();
    sweep_dir(conn, cache_dir, &mut stats)?;

    // Dangling rows first so phantom bytes cannot crowd real files past the cap.
    let dangling: Vec<String> = {
        let mut stmt = conn.prepare("SELECT url, local_path FROM cache_index")?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        rows.into_iter()
            .filter(|(_, p)| !Path::new(p).is_file())
            .map(|(u, _)| u)
            .collect()
    };
    for url in dangling {
        conn.execute("DELETE FROM cache_index WHERE url = ?1", [&url])?;
        stats.removed += 1;
    }

    let total = index_total(conn)?;
    let cap_i = i64::try_from(cap_bytes).unwrap_or(i64::MAX);
    let over = total.saturating_sub(cap_i);
    if over > 0 {
        // Victim order = evict order; a row goes while the strictly older rows
        // alone still leave the cache over the cap (keeps the newest prefix).
        let victims: Vec<(String, String, i64)> = {
            let mut stmt = conn.prepare(
                "WITH ordered AS (
                   SELECT url, local_path, bytes,
                          SUM(bytes) OVER (
                            ORDER BY last_access ASC, hit_count ASC, url ASC
                          ) - bytes AS older_bytes
                   FROM cache_index
                 )
                 SELECT url, local_path, bytes FROM ordered
                 WHERE older_bytes < ?1 ORDER BY older_bytes ASC",
            )?;
            let rows: Vec<(String, String, i64)> = stmt
                .query_map([over], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<rusqlite::Result<_>>()?;
            rows
        };
        for (url, ..) in &victims {
            conn.execute("DELETE FROM cache_index WHERE url = ?1", [url])?;
            stats.removed += 1;
        }
        for (_, path, bytes) in victims {
            // NotFound → already gone; open-elsewhere → orphan sweep later.
            if std::fs::remove_file(&path).is_ok() {
                stats.freed_bytes += bytes as u64;
            }
        }
    }
    stats.remaining_bytes = index_total(conn)?.max(0) as u64;
    Ok(stats)
}

/// Sweep the cache dir: `.part-*` temp files older than 1 h (active
/// downloads are fresh) and finals older than 5 min that no row references
/// (the tee rename→insert window is microseconds, so live files never match).
fn sweep_dir(conn: &Connection, dir: &Path, stats: &mut EvictStats) -> rusqlite::Result<()> {
    let referenced: std::collections::HashSet<String> = {
        let mut stmt = conn.prepare("SELECT local_path FROM cache_index")?;
        let paths: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        paths.into_iter().collect()
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(()); // cache dir not created yet
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let age = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .map(|d| d.as_secs())
            .unwrap_or(0); // unreadable mtime → keep (never kill a live temp)
        let part = name.starts_with(".part-");
        let stale = if part {
            age > PART_MAX_AGE_SECS
        } else {
            !name.starts_with('.')
                && age > ORPHAN_MIN_AGE_SECS
                && !referenced.contains(path.to_string_lossy().as_ref())
        };
        if !stale || std::fs::remove_file(&path).is_err() {
            continue;
        }
        stats.freed_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        if part {
            stats.removed_parts += 1;
        } else {
            stats.removed_orphans += 1;
        }
    }
    Ok(())
}
