//! P5.4 tests (split from `enrich_tests`): end-to-end `match_track` over a
//! mock `MbTransport` — parse → fielded query → pick → release pick, the
//! second identical run must be meta_cache-only (zero transport calls),
//! and a no-score title returns `Ok(None)` (row untouched).

use std::sync::{Arc, Mutex};

use crate::db;
use crate::meta::enrich::{match_track, TrackMeta};
use crate::meta::mb_client::{MbClient, MbTransport};
use crate::meta::rate_limit::RateLimitQueue;
use crate::resolver::{BoxFuture, SharedDb};

const MATCH_JSON: &str = r#"{
  "count": 1, "offset": 0,
  "recordings": [{
    "id": "rec-hit", "score": "100", "title": "back to friends",
    "length": 199032,
    "artist-credit": [{ "name": "sombr", "artist": { "name": "sombr" } }],
    "releases": [
      { "id": "rel-bootleg", "status": "Bootleg", "date": "2024-01-01" },
      { "id": "rel-off", "status": "Official", "date": "2025-06-01" },
      { "id": "rel-off-early", "status": "Official", "date": "2024-11-30" }
    ]
  }]
}"#;

fn track(title: &str, artist: &str, dur: Option<i64>) -> TrackMeta {
    TrackMeta {
        title: title.to_owned(),
        artist: artist.to_owned(),
        duration_ms: dur,
    }
}

struct CountingTransport {
    calls: Arc<Mutex<Vec<String>>>,
    body: String,
}

impl MbTransport for CountingTransport {
    fn get<'a>(&'a self, url: &'a str, _ua: &'a str) -> BoxFuture<'a, Result<String, String>> {
        self.calls.lock().unwrap().push(url.to_owned());
        let body = self.body.clone();
        Box::pin(async move { Ok(body) })
    }
}

#[tokio::test]
async fn match_track_picks_release_and_second_run_is_cache_only() {
    let db: SharedDb = Arc::new(Mutex::new(db::open_memory().unwrap()));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let client = MbClient::new(
        CountingTransport {
            calls: Arc::clone(&calls),
            body: MATCH_JSON.to_owned(),
        },
        RateLimitQueue::new(std::time::Duration::from_millis(1)),
        Arc::clone(&db),
    );
    let t = track("back to friends", "sombr", Some(199_079));

    let first = match_track(&client, &t).await.expect("decode").expect("must match");
    assert_eq!(first.recording_id, "rec-hit");
    assert_eq!(first.release_id.as_deref(), Some("rel-off-early"));
    assert_eq!(calls.lock().unwrap().len(), 1, "first run = one transport call");

    // Identical track → same fielded query → meta_cache only (zero calls).
    let second = match_track(&client, &t).await.expect("decode");
    assert_eq!(second, Some(first));
    assert_eq!(calls.lock().unwrap().len(), 1, "repeat must be cache-only");

    // A title that scores nothing → Ok(None): the command writes nothing.
    let none = match_track(&client, &track("Totally Other Song", "sombr", None))
        .await
        .expect("decode");
    assert_eq!(none, None);
}
