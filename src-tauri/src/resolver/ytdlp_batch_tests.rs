//! P20/D34 tests: batched yt-dlp — arg concat, doc→query mapping (echoed
//! id FIFO, positional fallback, missing docs), shared pick semantics, and
//! the spawn-failure latch. Canned JSON only — no network.

use crate::resolver::ytdlp::{pick_candidate, Entry};
use crate::resolver::ytdlp_batch::{build_batch_args, parse_batch, run_batch, BatchSlot};
use crate::resolver::ytdlp_args::search_term;
use crate::resolver::Query;

fn query(artist: &str, title: &str) -> Query {
    Query {
        artist: artist.to_owned(),
        title: title.to_owned(),
        album: None,
        duration_ms: None,
    }
}

fn entry(title: &str, artist: &str, duration: f64, url: &str) -> String {
    format!(
        "{{\"title\":\"{title}\",\"uploader\":\"{artist}\",\"duration\":{duration},\"ext\":\"m4a\",\"url\":\"{url}\"}}"
    )
}

fn doc(id: &str, entry_json: &str) -> String {
    format!("{{\"id\":\"{id}\",\"title\":\"{id}\",\"entries\":[{entry_json}]}}")
}

fn errs_of(slots: &[BatchSlot]) -> Vec<Option<String>> {
    slots
        .iter()
        .map(|s| match s {
            BatchSlot::Hit(_) => None,
            BatchSlot::Miss(m) => Some(m.clone()),
        })
        .collect()
}

fn hit_url(slot: &BatchSlot) -> String {
    match slot {
        BatchSlot::Hit(c) => c.url_or_path.clone(),
        BatchSlot::Miss(m) => panic!("expected hit, got miss: {m}"),
    }
}

#[test]
fn batch_args_repeat_full_flag_set_per_input() {
    let qs = [query("Arijit Singh", "Apna Bana Le"), query("AP Dhillon", "Excuses")];
    let args = build_batch_args(&qs);
    assert_eq!(args.len(), 18);
    assert_eq!(args.iter().filter(|a| *a == "--dump-single-json").count(), 2);
    assert!(args.contains(&"ytsearch3:Arijit Singh - Apna Bana Le".to_owned()));
    assert!(args.contains(&"ytsearch3:AP Dhillon - Excuses".to_owned()));
}

#[test]
fn parse_maps_docs_by_echoed_id_in_query_order() {
    let qs = [
        query("Arijit Singh", "Apna Bana Le"),
        query("Arijit Singh", "Tum Hi Ho"),
    ];
    let e0 = entry("Apna Bana Le", "Arijit Singh", 225.0, "https://media.example/a.m4a");
    let e1 = entry("Tum Hi Ho", "Arijit Singh", 240.0, "https://media.example/b.m4a");
    // Output order is shuffled vs the queries on purpose.
    let stdout = [
        doc(&search_term(&qs[1]), &e1),
        doc(&search_term(&qs[0]), &e0),
    ]
    .join("\n");
    let slots = parse_batch(&stdout, &qs);
    assert_eq!(slots.len(), 2);
    assert_eq!(hit_url(&slots[0]), "https://media.example/a.m4a");
    assert_eq!(hit_url(&slots[1]), "https://media.example/b.m4a");
    match &slots[0] {
        BatchSlot::Hit(c) => {
            assert_eq!(c.source, "ytdlp");
            assert_eq!(c.mime.as_deref(), Some("audio/mp4"));
        }
        BatchSlot::Miss(_) => unreachable!(),
    }
}

#[test]
fn parse_duplicate_terms_both_map_fifo() {
    let q = query("Same Artist", "Same Song");
    let qs = [q.clone(), q];
    let e = entry("Same Song", "Same Artist", 200.0, "https://media.example/dup.m4a");
    let stdout = [
        doc(&search_term(&qs[0]), &e),
        doc(&search_term(&qs[0]), &e),
    ]
    .join("\n");
    let slots = parse_batch(&stdout, &qs);
    let errs = errs_of(&slots);
    assert!(errs[0].is_none(), "{errs:?}");
    assert!(errs[1].is_none(), "{errs:?}");
}

#[test]
fn parse_missing_doc_is_honest_miss() {
    let qs = [query("First", "Gone"), query("Second", "Here")];
    let e = entry("Here", "Second", 180.0, "https://media.example/here.m4a");
    // Doc exists only for the second query.
    let stdout = doc(&search_term(&qs[1]), &e);
    let slots = parse_batch(&stdout, &qs);
    let errs = errs_of(&slots);
    assert!(matches!(&errs[0], Some(m) if m.contains("no result doc")), "{errs:?}");
    assert!(errs[1].is_none(), "{errs:?}");
}

#[test]
fn parse_positional_fallback_when_ids_stop_echoing() {
    let qs = [query("ArtistA", "One"), query("ArtistB", "Two")];
    let e0 = entry("One", "ArtistA", 100.0, "https://media.example/one.m4a");
    let e1 = entry("Two", "ArtistB", 110.0, "https://media.example/two.m4a");
    let stdout = [
        doc("drifted-id-1", &e0),
        doc("drifted-id-2", &e1),
    ]
    .join("\n");
    let slots = parse_batch(&stdout, &qs);
    assert_eq!(hit_url(&slots[0]), "https://media.example/one.m4a");
    assert_eq!(hit_url(&slots[1]), "https://media.example/two.m4a");
}

#[test]
fn pick_candidate_accepts_match_and_rejects_duration_miss() {
    let q = Query {
        artist: "Arijit Singh".to_owned(),
        title: "Apna Bana Le".to_owned(),
        album: None,
        duration_ms: Some(225_000),
    };
    let ok = vec![Entry {
        title: Some("Apna Bana Le".to_owned()),
        channel: None,
        uploader: Some("Arijit Singh".to_owned()),
        duration: Some(225.0),
        ext: Some("m4a".to_owned()),
        url: Some("https://media.example/a.m4a".to_owned()),
        webpage_url: None,
    }];
    assert!(pick_candidate(ok, &q).is_ok());

    let too_far = vec![Entry {
        title: Some("Apna Bana Le".to_owned()),
        channel: None,
        uploader: Some("Arijit Singh".to_owned()),
        duration: Some(400.0),
        ext: Some("m4a".to_owned()),
        url: Some("https://media.example/b.m4a".to_owned()),
        webpage_url: None,
    }];
    let err = pick_candidate(too_far, &q).unwrap_err().to_string();
    assert!(err.contains("no usable YouTube video"), "{err}");
}

#[test]
fn run_batch_spawn_failure_latches_all_slots() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let qs = [query("A", "One"), query("B", "Two")];
    let out = rt.block_on(run_batch(
        std::path::Path::new("C:\\definitely\\missing\\adios-nonexistent-ytdlp.exe"),
        &qs,
    ));
    assert!(out.latched);
    assert_eq!(out.slots.len(), 2);
    for s in &out.slots {
        match s {
            BatchSlot::Miss(m) => assert!(m.contains("spawn failed"), "{m}"),
            BatchSlot::Hit(_) => panic!("spawn failure must not produce hits"),
        }
    }
}
