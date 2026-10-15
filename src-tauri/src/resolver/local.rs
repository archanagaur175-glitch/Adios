//! LocalFile resolver (Invariant 3): maps a library query to an on-disk
//! path; the loopback server streams it. No network, no yt-dlp.

use std::future::ready;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rusqlite::Connection;

use super::score::{score, MIN_ACCEPT_CONFIDENCE, ScoreOutcome};
use super::{BoxFuture, Candidate, Health, HealthStatus, Query, ResolveError, Resolver, SharedDb};
use crate::server::range::mime_for_path;

pub struct LocalFileResolver {
    db: SharedDb,
}

impl LocalFileResolver {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    fn health_blocking(&self) -> Health {
        let started = Instant::now();
        let status = match self.db.lock() {
            Ok(conn) => match conn.query_row("SELECT 1", [], |_| Ok(())) {
                Ok(_) => HealthStatus::Ok,
                Err(_) => HealthStatus::Down,
            },
            Err(_) => HealthStatus::Down,
        };
        Health {
            status,
            latency_ms: started.elapsed().as_millis() as u32,
        }
    }

    fn resolve_blocking(&self, query: &Query) -> Result<Candidate, ResolveError> {
        let conn = self.db.lock().map_err(|e| ResolveError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT path, artist, title, duration_ms FROM tracks
                 WHERE path IS NOT NULL",
            )
            .map_err(|e| ResolveError::Database(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                ))
            })
            .map_err(|e| ResolveError::Database(e.to_string()))?;

        let mut best: Option<Candidate> = None;
        for row in rows {
            let (path, artist, title, duration_ms) =
                row.map_err(|e| ResolveError::Database(e.to_string()))?;
            if let ScoreOutcome::Accept { confidence } =
                score(query, &title, &artist, duration_ms)
            {
                let better = best
                    .as_ref()
                    .is_none_or(|b| confidence > b.confidence);
                if better && confidence >= MIN_ACCEPT_CONFIDENCE {
                    let mime = mime_for_path(std::path::Path::new(&path));
                    best = Some(Candidate {
                        url_or_path: path,
                        mime: Some(mime.to_owned()),
                        expires_at: None,
                        source: "local".to_owned(),
                        confidence,
                    });
                }
            }
        }
        best.ok_or(ResolveError::NotFound)
    }
}

impl Resolver for LocalFileResolver {
    fn name(&self) -> &'static str {
        "local"
    }

    fn health<'a>(&'a self) -> BoxFuture<'a, Health> {
        Box::pin(ready(self.health_blocking()))
    }

    fn resolve<'a>(&'a self, query: &'a Query) -> BoxFuture<'a, Result<Candidate, ResolveError>> {
        // Blocking rusqlite work happens eagerly; the future is just the result
        // (keeps the future Send despite the MutexGuard).
        let outcome = self.resolve_blocking(query);
        Box::pin(ready(outcome))
    }
}
