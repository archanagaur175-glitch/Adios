//! P4.4 tests: meta_cache write-back helpers + payload-only cache read.

use std::sync::{Arc, Mutex};

use futures::FutureExt;

use crate::db::open_memory;
use crate::resolver::cache::{invalidate_resolution, put_resolution, CacheResolver};
use crate::resolver::{Candidate, Query, ResolveError, Resolver};

fn query(title: &str, artist: &str, duration_ms: Option<i64>) -> Query {
    Query {
        artist: artist.to_owned(),
        title: title.to_owned(),
        album: None,
        duration_ms,
    }
}

fn candidate(url: &str, source: &str, confidence: f32) -> Candidate {
    Candidate {
        url_or_path: url.to_owned(),
        mime: Some("audio/mp4".to_owned()),
        expires_at: None,
        source: source.to_owned(),
        confidence,
    }
}

#[test]
fn put_resolution_serves_payload_only_then_invalidate_misses() {
    let db = Arc::new(Mutex::new(open_memory().unwrap()));
    let dir = std::env::temp_dir().join(format!("adios-cwb-{}", uuid::Uuid::new_v4()));
    let q = query("Husn", "Anuv Jain", Some(180_000));

    put_resolution(&db, &q, &candidate("https://cdn.example/v?id=1", "ytdlp", 0.75)).unwrap();

    let resolver = CacheResolver::new(db.clone(), dir.clone());
    let hit = resolver.resolve(&q).now_or_never().unwrap().unwrap();
    assert_eq!(hit.url_or_path, "https://cdn.example/v?id=1");
    assert_eq!(hit.source, "ytdlp");
    assert_eq!(hit.confidence, 0.75);
    assert_eq!(hit.mime.as_deref(), Some("audio/mp4"));

    invalidate_resolution(&db, &q).unwrap();
    assert!(matches!(
        resolver.resolve(&q).now_or_never().unwrap(),
        Err(ResolveError::NotFound)
    ));
}

#[test]
fn put_resolution_is_idempotent_and_ignores_local_paths() {
    let db = Arc::new(Mutex::new(open_memory().unwrap()));
    let q = query("Husn", "Anuv Jain", Some(180_000));

    put_resolution(&db, &q, &candidate("https://cdn.example/v?id=1", "ytdlp", 0.7)).unwrap();
    put_resolution(&db, &q, &candidate("https://cdn.example/v?id=2", "piped", 0.8)).unwrap();

    let resolver = CacheResolver::new(db.clone(), std::env::temp_dir().join("adios-cwb-skip"));
    let hit = resolver.resolve(&q).now_or_never().unwrap().unwrap();
    assert_eq!(hit.url_or_path, "https://cdn.example/v?id=2", "upsert wins");

    // Local paths are never written back → that query stays unresolved.
    let local_query = query("Apna Bana Le", "Arijit Singh", Some(261_000));
    put_resolution(&db, &local_query, &candidate("C:/music/husn.mp3", "local", 1.0)).unwrap();
    assert!(matches!(
        resolver.resolve(&local_query).now_or_never().unwrap(),
        Err(ResolveError::NotFound)
    ));
}
