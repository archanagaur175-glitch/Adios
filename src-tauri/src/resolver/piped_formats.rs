//! Audio-stream response parsers for Piped/Invidious (P4.4) — the pieces
//! that turn `/streams` or `/videos` responses into a direct URL + mime.
//! Shapes verified live 2026-10-15; see `piped_api.rs` doc for citations.

use serde::Deserialize;

/// Both APIs expose quality-labelled audio candidates; m4a always wins.
fn pick_audio(candidates: Vec<(String, String)>) -> Option<(String, String)> {
    candidates
        .into_iter()
        .max_by_key(|(_, mime)| mime.starts_with("audio/mp4") as u8)
}

pub fn parse_piped_streams(body: &str) -> Option<(String, String)> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Audio {
        url: String,
        mime_type: String,
        video_only: Option<bool>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Streams {
        audio_streams: Vec<Audio>,
    }
    let streams = serde_json::from_str::<Streams>(body).ok()?;
    let candidates: Vec<(String, String)> = streams
        .audio_streams
        .into_iter()
        .filter(|a| !a.video_only.unwrap_or(false))
        .map(|a| (a.url, a.mime_type))
        .collect();
    pick_audio(candidates)
}

pub fn parse_invidious_video(body: &str) -> Option<(String, String)> {
    #[derive(Deserialize)]
    struct Format {
        url: String,
        #[serde(rename = "type")]
        mime_type: String,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Video {
        adaptive_formats: Vec<Format>,
    }
    let video = serde_json::from_str::<Video>(body).ok()?;
    let candidates: Vec<(String, String)> = video
        .adaptive_formats
        .into_iter()
        .filter(|f| f.mime_type.starts_with("audio/"))
        .map(|f| {
            // "audio/mp4; codecs=\"mp4a.40.2\"" → "audio/mp4"
            let mime = f
                .mime_type
                .split(';')
                .next()
                .unwrap_or(&f.mime_type)
                .trim()
                .to_owned();
            (f.url, mime)
        })
        .collect();
    pick_audio(candidates)
}
