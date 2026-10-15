//! Shared helpers for server tests (both local and proxy suites).

use axum::http::header::{HeaderValue, ORIGIN};
use std::io::Write;

/// One shared temp file per test: 200 KiB of a repeating byte pattern.
pub fn temp_file(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("adios-server-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    let data: Vec<u8> = (0..200 * 1024).map(|i| (i % 251) as u8).collect();
    file.write_all(&data).unwrap();
    path
}

/// Start a server with an in-memory DB and a fresh temp cache dir.
pub fn test_start() -> (
    super::LoopbackServer,
    crate::resolver::SharedDb,
    std::path::PathBuf,
) {
    let conn = crate::db::open_memory().expect("open memory db");
    let db = crate::db::DbState::new(conn).shared();
    let cache_dir = std::env::temp_dir()
        .join("adios-server-tests")
        .join(format!("cache-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&cache_dir).unwrap();
    let server = super::start(db.clone(), cache_dir.clone()).expect("start");
    (server, db, cache_dir)
}

pub fn app_origin() -> HeaderValue {
    HeaderValue::from_static("http://localhost:1420")
}

pub fn origin_headers(value: HeaderValue) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(ORIGIN, value);
    headers
}

pub fn acao(res: &reqwest::Response) -> Option<String> {
    res.headers()
        .get(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}
