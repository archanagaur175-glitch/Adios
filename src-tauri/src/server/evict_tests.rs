//! P8.1 eviction tests: size cap respected, hit_count/LRU victim order,
//! and `cache_index` consistency (rows ↔ files) after every pass.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::{params, Connection};

use super::evict::{cache_cap_bytes, evict_to_cap};

const MB: i64 = 1024 * 1024;

fn fresh_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adios-evict-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn conn() -> Connection {
    crate::db::open_memory().unwrap()
}

/// Create a `size_mb`-byte file + its `cache_index` row with fixed LRU fields.
fn seed(
    conn: &Connection,
    dir: &Path,
    name: &str,
    size_mb: i64,
    last_access: &str,
    hit_count: i64,
) -> String {
    let path = dir.join(name);
    std::fs::write(&path, vec![0u8; (size_mb * MB) as usize]).unwrap();
    let url = format!("https://seed.invalid/{name}");
    conn.execute(
        "INSERT INTO cache_index (url, local_path, mime, bytes, last_access, hit_count)
         VALUES (?1, ?2, 'audio/mp4', ?3, ?4, ?5)",
        params![url, path.to_string_lossy().to_string(), size_mb * MB, last_access, hit_count],
    )
    .unwrap();
    url
}

fn set_mtime(path: &Path, age_secs: u64) {
    let file = std::fs::File::options().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - Duration::from_secs(age_secs))
        .unwrap();
}

fn index_urls(conn: &Connection) -> Vec<String> {
    let mut stmt = conn.prepare("SELECT url FROM cache_index ORDER BY url").unwrap();
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<String>>>();
    rows.unwrap()
}

fn index_total(conn: &Connection) -> i64 {
    let v: i64 = conn
        .query_row("SELECT COALESCE(SUM(bytes), 0) FROM cache_index", [], |r| r.get(0))
        .unwrap();
    v
}

/// The plan gate in one predicate: every remaining row points at a real file.
fn index_consistent(conn: &Connection) -> bool {
    let mut stmt = conn.prepare("SELECT local_path FROM cache_index").unwrap();
    let paths = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<String>>>()
        .unwrap();
    paths.iter().all(|p| Path::new(p).is_file())
}

#[test]
fn size_cap_respected_lru_evicted_first() {
    let dir = fresh_dir();
    let c = conn();
    seed(&c, &dir, "a.mp4", 10, "2026-01-01 00:00:00", 9); // oldest — high hits
    seed(&c, &dir, "b.mp4", 10, "2026-01-02 00:00:00", 0);
    seed(&c, &dir, "c.mp4", 10, "2026-01-03 00:00:00", 0);
    seed(&c, &dir, "d.mp4", 10, "2026-01-04 00:00:00", 0); // newest

    let stats = evict_to_cap(&c, &dir, (25 * MB) as u64).unwrap();

    // LRU wins over hit_count: the two oldest go despite a's 9 hits; the
    // newest prefix (20 MB) stays and the cap holds.
    assert_eq!(stats.removed, 2);
    assert_eq!(stats.freed_bytes, (20 * MB) as u64);
    assert_eq!(stats.remaining_bytes, (20 * MB) as u64);
    assert!(index_total(&c) <= 25 * MB);
    assert_eq!(
        index_urls(&c),
        vec![
            "https://seed.invalid/c.mp4".to_string(),
            "https://seed.invalid/d.mp4".to_string()
        ]
    );
    assert!(!dir.join("a.mp4").exists() && !dir.join("b.mp4").exists());
    assert!(dir.join("c.mp4").exists() && dir.join("d.mp4").exists());
    assert!(index_consistent(&c));
}

#[test]
fn equal_last_access_evicts_lowest_hit_count_first() {
    let dir = fresh_dir();
    let c = conn();
    seed(&c, &dir, "x.mp4", 10, "2026-01-01 00:00:00", 5); // survives (tie-break)
    seed(&c, &dir, "y.mp4", 10, "2026-01-01 00:00:00", 1); // evicted first
    seed(&c, &dir, "z.mp4", 10, "2026-01-01 00:00:00", 3);

    let stats = evict_to_cap(&c, &dir, (15 * MB) as u64).unwrap();

    assert_eq!(stats.removed, 2);
    assert_eq!(index_urls(&c), vec!["https://seed.invalid/x.mp4".to_string()]);
    assert!(index_consistent(&c));
}

#[test]
fn dangling_rows_dropped_index_stays_consistent() {
    let dir = fresh_dir();
    let c = conn();
    seed(&c, &dir, "keep.mp4", 5, "2026-02-01 00:00:00", 1);
    c.execute(
        "INSERT INTO cache_index (url, local_path, mime, bytes, last_access, hit_count)
         VALUES ('https://seed.invalid/gone', ?1, 'audio/mp4', 9, '2026-01-01 00:00:00', 0)",
        [dir.join("gone.mp4").to_string_lossy().to_string()],
    )
    .unwrap(); // row without a file

    let stats = evict_to_cap(&c, &dir, u64::MAX).unwrap();

    // Dangling row removed under a huge cap (no real file to evict), and its
    // phantom bytes reclaim nothing from disk.
    assert_eq!(stats.removed, 1);
    assert_eq!(stats.freed_bytes, 0);
    assert_eq!(index_urls(&c), vec!["https://seed.invalid/keep.mp4".to_string()]);
    assert!(index_consistent(&c));
}

#[test]
fn stale_temp_parts_swept_active_kept() {
    let dir = fresh_dir();
    let c = conn();
    let stale = dir.join(".part-stale");
    std::fs::write(&stale, vec![7u8; 2048]).unwrap();
    set_mtime(&stale, 2 * 3600);
    let active = dir.join(".part-active");
    std::fs::write(&active, vec![7u8; 1024]).unwrap();

    let stats = evict_to_cap(&c, &dir, u64::MAX).unwrap();

    assert_eq!(stats.removed_parts, 1);
    assert_eq!(stats.freed_bytes, 2048);
    assert!(!stale.exists());
    assert!(active.exists()); // fresh temp = live download, never touched
}

#[test]
fn orphaned_finals_swept_only_when_old() {
    let dir = fresh_dir();
    let c = conn();
    seed(&c, &dir, "live.mp4", 1, "2026-03-01 00:00:00", 0); // referenced → kept
    let old = dir.join("orphan-old.mp4");
    std::fs::write(&old, vec![9u8; 4096]).unwrap();
    set_mtime(&old, 600); // > 5 min: crash leftover
    let young = dir.join("orphan-new.mp4");
    std::fs::write(&young, vec![9u8; 4096]).unwrap(); // rename→insert race guard

    let stats = evict_to_cap(&c, &dir, u64::MAX).unwrap();

    assert_eq!(stats.removed_orphans, 1);
    assert!(!old.exists());
    assert!(young.exists());
    assert!(dir.join("live.mp4").exists());
    assert!(index_consistent(&c));
}

#[test]
fn cap_setting_read_with_floor_and_default() {
    let c = conn();
    assert_eq!(cache_cap_bytes(&c), 512 * MB as u64); // missing → default
    c.execute(
        "INSERT INTO settings (key, value_json) VALUES ('cacheSizeMb', '256')",
        [],
    )
    .unwrap();
    assert_eq!(cache_cap_bytes(&c), 256 * MB as u64);
    c.execute("UPDATE settings SET value_json = '0'", []).unwrap();
    assert_eq!(cache_cap_bytes(&c), 16 * MB as u64); // floor
    c.execute("UPDATE settings SET value_json = 'garbage'", []).unwrap();
    assert_eq!(cache_cap_bytes(&c), 512 * MB as u64); // unreadable → default
}
