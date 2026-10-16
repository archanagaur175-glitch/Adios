//! P4.2 tests: yt-dlp arg construction (required) + JSON entry picking.
//! P9.7: parenthetical stripping, ytsearch3, multi-entry returns.
//! The real-network spike runs separately via the #[ignore] test below.

use crate::resolver::ytdlp::{pick_entries, YtDlpResolver};
use crate::resolver::ytdlp_args::{build_args, strip_parentheticals};
use crate::resolver::{HealthStatus, Query, Resolver};

fn query(artist: &str, title: &str) -> Query {
    Query {
        artist: artist.to_owned(),
        title: title.to_owned(),
        album: None,
        duration_ms: None,
    }
}

#[test]
fn build_args_searches_three_and_prefers_m4a() {
    let args = build_args(&query("Arijit Singh", "Apna Bana Le"));
    assert_eq!(args.last().unwrap(), "ytsearch3:Arijit Singh - Apna Bana Le");
    assert!(args.windows(2).any(|w| w == ["-f", "bestaudio[ext=m4a]/bestaudio"]));
    for flag in [
        "--no-playlist",
        "--no-warnings",
        "--skip-download",
        "--dump-single-json",
    ] {
        assert!(args.iter().any(|a| a == flag), "missing {flag}");
    }
    assert!(args
        .windows(2)
        .any(|w| w == ["--socket-timeout", "15"]));
}

#[test]
fn build_args_strips_parentheticals_from_query() {
    let args = build_args(&query("Artist (Live)", "Song (feat. Someone) [Remastered]"));
    assert_eq!(args.last().unwrap(), "ytsearch3:Artist - Song");
}

#[test]
fn strip_parentheticals_drops_nested_and_keeps_plain() {
    assert_eq!(strip_parentheticals("Song (Live)"), "Song");
    assert_eq!(strip_parentheticals("a (b (c) d) e"), "a e");
    assert_eq!(strip_parentheticals("Song [Remix] (Deluxe)"), "Song");
    assert_eq!(strip_parentheticals("Plain Title"), "Plain Title");
    assert_eq!(strip_parentheticals("Unbalanced (paren"), "Unbalanced");
}

#[test]
fn pick_entries_reads_ytsearch_entries_array() {
    let json = r#"{"_type":"playlist","entries":[
        {"id":"abc","title":"Apna Bana Le","channel":"Zee Music Company",
         "duration":204.0,"ext":"m4a","url":"https://rr1---sn-x.googlevideo.com/videoplayback?id=1"},
        {"id":"def","title":"Apna Bana Le (Live)","channel":"Other",
         "duration":200.0,"ext":"webm","url":"https://example/2"}
    ]}"#;
    let entries = pick_entries(json);
    assert_eq!(entries.len(), 2, "all entries returned in relevance order");
    assert_eq!(entries[0].title.as_deref(), Some("Apna Bana Le"));
    assert_eq!(entries[0].channel.as_deref(), Some("Zee Music Company"));
    assert_eq!(entries[0].duration, Some(204.0));
    assert_eq!(entries[0].ext.as_deref(), Some("m4a"));
}

#[test]
fn pick_entries_reads_direct_url_root_object() {
    let json = r#"{"id":"xyz","title":"Husn","uploader":"Anuv Jain","duration":180.0,
        "ext":"webm","webpage_url":"https://www.youtube.com/watch?v=xyz"}"#;
    let entries = pick_entries(json);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].uploader.as_deref(), Some("Anuv Jain"));
    assert_eq!(
        entries[0].webpage_url.as_deref(),
        Some("https://www.youtube.com/watch?v=xyz")
    );
    assert!(entries[0].url.is_none());
}

#[test]
fn pick_entries_returns_empty_on_garbage() {
    assert!(pick_entries("not json").is_empty());
    assert!(pick_entries(r#"{"entries":[]}"#).is_empty());
}

#[tokio::test]
async fn health_is_down_when_binary_missing() {
    let resolver = YtDlpResolver::new(crate::resolver::ytdlp::bin_handle(
        std::path::PathBuf::from("definitely-not-ytdlp-404"),
    ));
    let health = resolver.health().await;
    assert_eq!(health.status, HealthStatus::Down);
}

/// Manual spike: real YouTube resolve through the sidecar (network required).
/// Run: `cargo test spike_resolve -- --ignored --nocapture`
#[tokio::test]
#[ignore]
async fn spike_resolve_real_query() {
    let bin = std::env::var("ADIOS_YTDLP").unwrap_or_else(|_| "yt-dlp".to_owned());
    let resolver =
        YtDlpResolver::new(crate::resolver::ytdlp::bin_handle(std::path::PathBuf::from(bin)));
    let q = query("Arijit Singh", "Apna Bana Le");
    let result = resolver.resolve(&q).await;
    let ok = result.is_ok();
    match result {
        Ok(c) => println!(
            "[spike] hit source={} conf={:.2} mime={:?} url_host={}",
            c.source,
            c.confidence,
            c.mime,
            c.url_or_path.split('/').nth(2).unwrap_or("non-url")
        ),
        Err(e) => println!("[spike] miss: {e}"),
    }
    assert!(ok, "spike resolve failed");
}
