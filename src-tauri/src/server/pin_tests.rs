//! P12 pin tests: pinned cache rows survive LRU eviction; the pin/batch
//! helpers behave (idempotent pin, empty-batch short circuit).

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use super::cache::{cache_index_pin, cache_index_put};
use super::evict::evict_to_cap;

const MB: i64 = 1024 * 1024;

fn fresh_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("adios-pin-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn conn() -> Connection {
    crate::db::open_memory().unwrap()
}

/// Seed a `size_mb`-byte file + row; pin when `pinned`.
fn seed(conn: &Connection, dir: &Path, name: &str, size_mb: i64, last_access: &str, pinned: bool) {
    let path = dir.join(name);
    std::fs::write(&path, vec![0u8; (size_mb * MB) as usize]).unwrap();
    let url = format!("https://seed.invalid/{name}");
    conn.execute(
        "INSERT INTO cache_index (url, local_path, mime, bytes, last_access, hit_count, pinned)
         VALUES (?1, ?2, 'audio/mp4', ?3, ?4, 0, ?5)",
        params![url, path.to_string_lossy().to_string(), size_mb * MB, last_access, pinned],
    )
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

#[test]
fn pinned_row_survives_cap_pressure() {
    let dir = fresh_dir();
    let c = conn();
    seed(&c, &dir, "pinned.mp4", 10, "2026-01-01 00:00:00", true); // oldest AND pinned
    seed(&c, &dir, "mid.mp4", 10, "2026-01-02 00:00:00", false);
    seed(&c, &dir, "new.mp4", 10, "2026-01-03 00:00:00", false);

    let stats = evict_to_cap(&c, &dir, (15 * MB) as u64).unwrap();

    // 30 MB total, 15 MB cap: both unpinned rows go (they alone are 20 MB
    // of over-cap pressure). The pinned row is never a victim even though
    // it is the LRU — remaining may exceed the cap by design.
    assert_eq!(stats.removed, 2);
    assert_eq!(index_urls(&c), vec!["https://seed.invalid/pinned.mp4".to_string()]);
    assert!(dir.join("pinned.mp4").exists());
    assert!(!dir.join("mid.mp4").exists() && !dir.join("new.mp4").exists());
}

#[test]
fn all_pinned_nothing_evicted() {
    let dir = fresh_dir();
    let c = conn();
    seed(&c, &dir, "a.mp4", 10, "2026-01-01 00:00:00", true);
    seed(&c, &dir, "b.mp4", 10, "2026-01-02 00:00:00", true);

    let stats = evict_to_cap(&c, &dir, (1 * MB) as u64).unwrap();

    assert_eq!(stats.removed, 0);
    assert_eq!(stats.remaining_bytes, (20 * MB) as u64);
    assert_eq!(index_urls(&c).len(), 2);
}

#[test]
fn dangling_pinned_row_still_dropped() {
    let dir = fresh_dir();
    let c = conn();
    seed(&c, &dir, "keep.mp4", 5, "2026-02-01 00:00:00", true);
    c.execute(
        "INSERT INTO cache_index (url, local_path, mime, bytes, last_access, hit_count, pinned)
         VALUES ('https://seed.invalid/gone', ?1, 'audio/mp4', 9, '2026-01-01 00:00:00', 0, 1)",
        [dir.join("missing.mp4").to_string_lossy().to_string()],
    )
    .unwrap();

    let stats = evict_to_cap(&c, &dir, u64::MAX).unwrap();

    // Pin protects against the CAP, not against a vanished file.
    assert_eq!(stats.removed, 1);
    assert_eq!(index_urls(&c), vec!["https://seed.invalid/keep.mp4".to_string()]);
}

#[test]
fn pin_helper_idempotent() {
    let dir = fresh_dir();
    let c = conn();
    cache_index_put(&c, "https://x/1", dir.join("1.mp4").to_str().unwrap(), Some("audio/mp4"), 10)
        .unwrap();

    assert_eq!(cache_index_pin(&c, "https://x/1").unwrap(), 1);
    assert_eq!(cache_index_pin(&c, "https://x/1").unwrap(), 1, "idempotent");
    let pinned: i64 = c
        .query_row("SELECT pinned FROM cache_index WHERE url = 'https://x/1'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(pinned, 1);
}
