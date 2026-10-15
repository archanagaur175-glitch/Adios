//! P5.1 live network check (ignored by default): first search fetches +
//! caches, repeat is cache-only with exactly one rate slot used.
//! Run: `cargo test mb_live -- --ignored --nocapture`

use std::sync::{Arc, Mutex};

use crate::db::open_memory;
use crate::meta::mb_client::MbClient;
use crate::meta::mb_types::{MbArtist, SearchKind};
use crate::resolver::SharedDb;

#[tokio::test]
#[ignore]
async fn mb_live_search_roundtrip_then_cache_only() {
    let db: SharedDb = Arc::new(Mutex::new(open_memory().unwrap()));
    let c = MbClient::with_default_transport(Arc::clone(&db));

    let page = c
        .search::<MbArtist>(SearchKind::Artist, "The Weeknd")
        .await
        .expect("live MB search");
    assert!(page.count > 0, "expected live results");
    let first_id = page.items[0].id.clone();
    println!(
        "[live] count={} first={:?} score={:?}",
        page.count, page.items[0].name, page.items[0].score
    );

    let again = c
        .search::<MbArtist>(SearchKind::Artist, "the  weeknd")
        .await
        .expect("cache read");
    assert_eq!(again.items[0].id, first_id);
    assert_eq!(
        c.queue().acquired(),
        1,
        "repeat served from meta_cache without another rate slot"
    );
}
