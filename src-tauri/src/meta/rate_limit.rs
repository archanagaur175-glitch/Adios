//! Global 1-request-per-second queue for MusicBrainz.
//!
//! Docs: "each of their client applications never make more than ONE call
//! per second" (musicbrainz.org/doc/MusicBrainz_API — Rate Limiting section).
//! The mutex is held across the sleep, so slot starts are strictly
//! serialized: request N may START at earliest `slot(N-1) + interval`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

pub struct RateLimitQueue {
    interval: Duration,
    /// Next allowed request start (None before the first request).
    slot: Mutex<Option<Instant>>,
    acquired: AtomicUsize,
}

impl RateLimitQueue {
    pub fn new(interval: Duration) -> Arc<Self> {
        Arc::new(Self {
            interval,
            slot: Mutex::new(None),
            acquired: AtomicUsize::new(0),
        })
    }

    /// The app-wide default: one request per second.
    pub fn one_per_second() -> Arc<Self> {
        Self::new(Duration::from_secs(1))
    }

    /// Block until this caller may start its request, then reserve the slot.
    /// The FIRST request returns immediately (no cold-start penalty).
    pub async fn wait(&self) {
        let mut guard = self.slot.lock().await;
        let now = Instant::now();
        let start = match *guard {
            Some(prev) => (prev + self.interval).max(now),
            None => now,
        };
        if start > now {
            tokio::time::sleep_until(tokio::time::Instant::from_std(start)).await;
        }
        *guard = Some(start);
        self.acquired.fetch_add(1, Ordering::SeqCst);
    }

    /// How many slots have been acquired (test/observability hook).
    #[cfg(test)]
    pub fn acquired(&self) -> usize {
        self.acquired.load(Ordering::SeqCst)
    }
}
