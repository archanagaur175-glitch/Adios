//! P4.2 tests: yt-dlp arg construction (required) + JSON entry picking.
//! The real-network spike runs separately via the #[ignore] test below.

use crate::resolver::ytdlp::{build_args, pick_entry, YtDlpResolver};
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
fn build_args_searches_and_prefers_m4a() {
    let args = build_args(&query("Arijit Singh", "Apna Bana Le"));
    assert_eq!(args.last().unwrap(), "ytsearch1:Arijit Singh - Apna Bana Le");
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
fn pick_entry_reads_ytsearch_entries_array() {
    let json = r#"{"_type":"playlist","entries":[
        {"id":"abc","title":"Apna Bana Le","channel":"Zee Music Company",
         "duration":204.0,"ext":"m4a","url":"https://rr1---sn-x.googlevideo.com/videoplayback?id=1"}
    ]}"#;
    let entry = pick_entry(json).expect("entry");
    assert_eq!(entry.title.as_deref(), Some("Apna Bana Le"));
    assert_eq!(entry.channel.as_deref(), Some("Zee Music Company"));
    assert_eq!(entry.duration, Some(204.0));
    assert_eq!(entry.ext.as_deref(), Some("m4a"));
}

#[test]
fn pick_entry_reads_direct_url_root_object() {
    let json = r#"{"id":"xyz","title":"Husn","uploader":"Anuv Jain","duration":180.0,
        "ext":"webm","webpage_url":"https://www.youtube.com/watch?v=xyz"}"#;
    let entry = pick_entry(json).expect("entry");
    assert_eq!(entry.uploader.as_deref(), Some("Anuv Jain"));
    assert_eq!(entry.webpage_url.as_deref(), Some("https://www.youtube.com/watch?v=xyz"));
    assert!(entry.url.is_none());
}

#[test]
fn pick_entry_returns_none_on_garbage() {
    assert!(pick_entry("not json").is_none());
    assert!(pick_entry(r#"{"entries":[]}"#).is_none());
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
