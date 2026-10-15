//! P4.1 tests: Cache resolver + ResolverChain orchestration.

use std::sync::{Arc, Mutex};

use futures::FutureExt;

use crate::db::open_memory;
use crate::resolver::cache::{cache_key, CacheResolver};
use crate::resolver::local::LocalFileResolver;
use crate::resolver::{HealthStatus, Query, ResolveError, Resolver, ResolverChain};

fn query(title: &str, artist: &str, duration_ms: Option<i64>) -> Query {
    Query {
        artist: artist.to_owned(),
        title: title.to_owned(),
        album: None,
        duration_ms,
    }
}

fn temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("adios-resolver-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed_resolution(
    conn: &rusqlite::Connection,
    q: &Query,
    url: &str,
    local_path: &str,
    mime: &str,
) {
    let payload = format!(r#"{{"url":"{url}","mime":"{mime}"}}"#);
    conn.execute(
        "INSERT INTO meta_cache (cache_key, kind, payload_json) VALUES (?1, 'resolution', ?2)",
        rusqlite::params![cache_key(q), payload],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO cache_index (url, local_path, mime, bytes) VALUES (?1, ?2, ?3, 1000)",
        rusqlite::params![url, local_path, mime],
    )
    .unwrap();
}

#[test]
fn cache_resolver_serves_seeded_resolution_with_disk_file() {
    let dir = temp_dir();
    let file = dir.join("song.m4a");
    std::fs::write(&file, vec![0u8; 64]).unwrap();

    let db = Arc::new(Mutex::new(open_memory().unwrap()));
    let q = query("Husn", "Anuv Jain", Some(180_000));
    {
        let conn = db.lock().unwrap();
        seed_resolution(
            &conn,
            &q,
            "https://cdn.example/videoplayback?id=abc",
            file.to_str().unwrap(),
            "audio/mp4",
        );
    }
    let resolver = CacheResolver::new(db, dir.clone());

    let hit = resolver.resolve(&q).now_or_never().unwrap().unwrap();
    assert_eq!(hit.source, "cache");
    assert_eq!(hit.confidence, 1.0);
    assert_eq!(hit.mime.as_deref(), Some("audio/mp4"));
    assert_eq!(hit.url_or_path, file.to_str().unwrap());

    // Same query shape, different metadata → different key → miss.
    let other = query("Husn", "Anuv Jain", Some(181_000));
    assert!(matches!(
        resolver.resolve(&other).now_or_never().unwrap(),
        Err(ResolveError::NotFound)
    ));

    assert_eq!(
        resolver.health().now_or_never().unwrap().status,
        HealthStatus::Ok
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cache_resolver_misses_when_disk_file_gone() {
    let dir = temp_dir();
    let db = Arc::new(Mutex::new(open_memory().unwrap()));
    let q = query("Husn", "Anuv Jain", Some(180_000));
    {
        let conn = db.lock().unwrap();
        seed_resolution(
            &conn,
            &q,
            "https://cdn.example/v?id=xyz",
            dir.join("missing.m4a").to_str().unwrap(),
            "audio/mp4",
        );
    }
    let resolver = CacheResolver::new(db, dir.clone());
    assert!(matches!(
        resolver.resolve(&q).now_or_never().unwrap(),
        Err(ResolveError::NotFound)
    ));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn chain_prefers_local_and_reports_health_per_resolver() {
    let db = Arc::new(Mutex::new(open_memory().unwrap()));
    {
        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO tracks (source, path, artist, title, duration_ms)
             VALUES ('local', 'C:/music/husn.mp3', 'Anuv Jain', 'Husn', 180000)",
            [],
        )
        .unwrap();
    }
    let dir = temp_dir();
    let chain = ResolverChain::new(vec![
        Arc::new(LocalFileResolver::new(db.clone())),
        Arc::new(CacheResolver::new(db, dir.clone())),
    ]);

    let hit = futures::executor::block_on(chain.resolve(&query("Husn", "Anuv Jain", Some(180_000))))
        .unwrap();
    assert_eq!(hit.source, "local", "LocalFile must win over Cache");

    let health = futures::executor::block_on(chain.health());
    assert_eq!(health.len(), 2);
    assert_eq!(health[0].0, "local");
    assert_eq!(health[0].1.status, HealthStatus::Ok);
    assert_eq!(health[1].0, "cache");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn empty_chain_returns_not_found() {
    let chain = ResolverChain::new(vec![]);
    let err =
        futures::executor::block_on(chain.resolve(&query("Anything", "Anyone", None))).unwrap_err();
    assert!(matches!(err, ResolveError::NotFound));
}
