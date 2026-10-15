//! Piped/Invidious parser fixtures — shapes captured live 2026-10-15
//! (docs.piped.video, docs.invidious.io, api.piped.private.coffee,
//! invidious.f5.si); see BUILD_PLAN VERIFICATION LOG for the raw output.

use super::piped_api::*;
use super::piped_formats::{parse_invidious_video, parse_piped_streams};

#[test]
fn stats_probe_identifies_invidious() {
    let stats = r#"{"software":{"name":"invidious","version":"2026.01.1"}}"#;
    assert!(parse_stats_says_invidious(stats));
    assert!(!parse_stats_says_invidious(r#"{"software":{"name":"piped"}}"#));
    assert!(!parse_stats_says_invidious("<html>502 bad gateway</html>"));
}

#[test]
fn suggestions_probe_identifies_piped() {
    assert!(parse_is_str_list(r#"["apna bana le","apna bana le song"]"#));
    assert!(!parse_is_str_list(r#"{"error":"nope"}"#));
}

#[test]
fn piped_search_fixture_parses_stream_hits() {
    let body = r#"{"items":[
        {"url":"/watch?v=dQw4w9WgXcQ","type":"stream","title":"Apna Bana Le",
         "uploaderName":"Arijit Singh","duration":261,"views":123},
        {"url":"/channel/UC123","type":"channel","title":"Arijit Singh",
         "uploaderName":"","duration":0}
    ]}"#;
    let hits = parse_piped_search(body);
    assert_eq!(hits.len(), 1, "non-stream items are filtered");
    assert_eq!(hits[0].id, "dQw4w9WgXcQ");
    assert_eq!(hits[0].title, "Apna Bana Le");
    assert_eq!(hits[0].uploader, "Arijit Singh");
    assert_eq!(hits[0].duration_ms, Some(261_000));
}

#[test]
fn invidious_search_fixture_parses_video_hits() {
    let body = r#"[{"type":"video","title":"Husn","videoId":"abc123xyz",
        "author":"Anuv Jain","lengthSeconds":184,"viewCount":99},
        {"type":"playlist","title":"mix","videoId":"PL1","author":"x"}]"#;
    let hits = parse_invidious_search(body);
    assert_eq!(hits.len(), 1, "non-video items are filtered");
    assert_eq!(hits[0].id, "abc123xyz");
    assert_eq!(hits[0].uploader, "Anuv Jain");
    assert_eq!(hits[0].duration_ms, Some(184_000));
}

#[test]
fn piped_streams_prefers_m4a_and_skips_video_only() {
    let body = r#"{"audioStreams":[
        {"url":"https://a/videoplayback?webm","mimeType":"audio/webm; codecs=\"opus\"",
         "bitrate":160000,"videoOnly":false},
        {"url":"https://a/videoplayback?m4a","mimeType":"audio/mp4; codecs=\"mp4a.40.2\"",
         "bitrate":130000,"videoOnly":false},
        {"url":"https://a/videoplayback?vid","mimeType":"video/mp4",
         "bitrate":1000000,"videoOnly":true}
    ],"title":"t"}"#;
    let got = parse_piped_streams(body).expect("candidate");
    assert_eq!(got.0, "https://a/videoplayback?m4a", "m4a beats webm");
    assert_eq!(got.1, "audio/mp4; codecs=\"mp4a.40.2\"");
}

#[test]
fn invidious_video_strips_codec_and_prefers_mp4() {
    let body = r#"{"title":"t","adaptiveFormats":[
        {"type":"audio/webm; codecs=\"opus\"","url":"https://a/webm"},
        {"type":"audio/mp4; codecs=\"mp4a.40.2\"","url":"https://a/m4a"},
        {"type":"video/mp4; codecs=\"avc1\"","url":"https://a/video"}
    ]}"#;
    let got = parse_invidious_video(body).expect("candidate");
    assert_eq!(got.0, "https://a/m4a");
    assert_eq!(got.1, "audio/mp4", "codec suffix stripped");
}

#[test]
fn malformed_bodies_never_panic() {
    assert!(parse_piped_search("").is_empty());
    assert!(parse_invidious_search("{}").is_empty());
    assert!(parse_piped_streams("not json").is_none());
    assert!(parse_invidious_video("[]").is_none());
}

#[test]
fn encode_escapes_space_and_unicode() {
    assert_eq!(encode("apna bana le"), "apna%20bana%20le");
    assert_eq!(encode("Husn (Anuv Jain)"), "Husn%20%28Anuv%20Jain%29");
    assert_eq!(encode("प्यार"), "%E0%A4%AA%E0%A5%8D%E0%A4%AF%E0%A4%BE%E0%A4%B0");
    assert_eq!(encode("azAZ09-_.~"), "azAZ09-_.~");
}
