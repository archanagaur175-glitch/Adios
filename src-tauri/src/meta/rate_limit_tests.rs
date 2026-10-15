//! P5.1 tests: rate-limit queue spacing (plan requirement: 10 rapid requests
//! span >= 9 s at a 1 s interval).

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::meta::rate_limit::RateLimitQueue;

#[tokio::test]
async fn ten_rapid_requests_span_at_least_9_seconds() {
    let q = RateLimitQueue::one_per_second();
    let started = Instant::now();

    let mut handles = Vec::with_capacity(10);
    for _ in 0..10 {
        let q = Arc::clone(&q);
        handles.push(tokio::spawn(async move {
            q.wait().await;
            started.elapsed()
        }));
    }

    let mut finishes = Vec::with_capacity(10);
    for h in handles {
        finishes.push(h.await.expect("task join"));
    }
    finishes.sort();

    let first = finishes[0];
    let last = finishes[9];
    let span = last.saturating_sub(first);
    assert!(
        span >= Duration::from_secs(9),
        "10 requests finished within {span:?} of each other; expected >= 9s spacing (first={first:?}, last={last:?})"
    );
    // The first request must not wait (cold start is free).
    assert!(
        first < Duration::from_millis(500),
        "first request waited {first:?}; expected immediate start"
    );
    assert_eq!(q.acquired(), 10);
}

#[tokio::test]
async fn sequential_requests_are_at_least_one_second_apart() {
    let q = RateLimitQueue::one_per_second();
    let mut stamps = Vec::new();
    for _ in 0..3 {
        q.wait().await;
        stamps.push(Instant::now());
    }
    let gap1 = stamps[1].duration_since(stamps[0]);
    let gap2 = stamps[2].duration_since(stamps[1]);
    assert!(gap1 >= Duration::from_millis(950), "gap1 = {gap1:?}");
    assert!(gap2 >= Duration::from_millis(950), "gap2 = {gap2:?}");
}
