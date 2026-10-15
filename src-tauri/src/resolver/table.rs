//! In-memory resolution table (architecture §3.2): remote URL → original
//! `Query`, so the proxy can re-resolve once when an upstream URL 403s/
//! expires. Attempt accounting guarantees at most ONE automatic re-resolve
//! per explicit resolution: `retarget` carries the count forward so a chain
//! of bad URLs can never loop.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::Query;

#[derive(Debug, Clone)]
struct Entry {
    query: Query,
    attempts: u32,
}

/// Shared table managed by Tauri state and held by the loopback server.
#[derive(Default)]
pub struct ResolutionTable {
    map: Mutex<HashMap<String, Entry>>,
}

pub type SharedTable = Arc<ResolutionTable>;

impl ResolutionTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a fresh (re)resolution — resets the attempt counter. Called
    /// from the resolve command (an explicit user/FE action).
    pub fn register(&self, url: &str, query: &Query) {
        if let Ok(mut map) = self.map.lock() {
            map.insert(
                url.to_owned(),
                Entry {
                    query: query.clone(),
                    attempts: 0,
                },
            );
        }
    }

    /// The query that produced this URL, when known.
    pub fn query_for(&self, url: &str) -> Option<Query> {
        self.map
            .lock()
            .ok()?
            .get(url)
            .map(|entry| entry.query.clone())
    }

    /// Claim the single automatic re-resolve for this URL. Succeeds at most
    /// once until a later explicit `register` resets the counter.
    pub fn claim(&self, url: &str) -> bool {
        match self.map.lock() {
            Ok(mut map) => match map.get_mut(url) {
                Some(entry) if entry.attempts == 0 => {
                    entry.attempts = 1;
                    true
                }
                _ => false,
            },
            Err(_) => false,
        }
    }

    /// Point an entry at a fresh URL, CARRYING the attempt count forward:
    /// a re-resolve that produced another expiring URL gets no second try.
    pub fn retarget(&self, from: &str, to: &str, query: &Query) {
        if let Ok(mut map) = self.map.lock() {
            let attempts = map.remove(from).map(|e| e.attempts).unwrap_or(0);
            map.insert(
                to.to_owned(),
                Entry {
                    query: query.clone(),
                    attempts,
                },
            );
        }
    }
}
