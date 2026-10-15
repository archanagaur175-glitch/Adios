//! Piped/Invidious wire API: flavor probe + pure response parsers.
//! Shapes verified LIVE 2026-10-15 (citations in BUILD_PLAN VERIFICATION LOG):
//! - Piped: docs.piped.video/docs/api-documentation/ Â· probe api.piped.private.coffee
//!   `GET /suggestions?query=x` â†’ `["x", â€¦]` Â· `GET /search?q=x&filter=videos` â†’
//!   `{items:[{url:"/watch?v=ID",type:"stream",title,uploaderName,duration}]}` Â·
//!   `GET /streams/{id}` â†’ `{audioStreams:[{url,mimeType,bitrate,videoOnly}]}`
//! - Invidious: docs.invidious.io/api Â· probe invidious.f5.si (api âœ”):
//!   `GET /api/v1/stats` â†’ `{software:{name:"invidious"}}` Â·
//!   `GET /api/v1/search?q=x&type=video` â†’ `[{type,title,videoId,author,lengthSeconds}]` Â·
//!   `GET /api/v1/videos/{id}` â†’ `{adaptiveFormats:[{type:'audio/mp4; codecs="â€¦"',url}]}`
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Piped,
    Invidious,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub uploader: String,
    pub duration_ms: Option<i64>,
}

pub const USER_AGENT: &str = "Adios/0.1 (personal use)";

/// Flavor detection: Invidious `/api/v1/stats` first, then Piped `/suggestions`.
pub async fn probe(client: &reqwest::Client, base: &str) -> Option<Flavor> {
    let get = |url: String| {
        client
            .get(url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
    };
    if let Ok(res) = get(format!("{base}/api/v1/stats")).await {
        if res.status().is_success()
            && res
                .text()
                .await
                .ok()
                .is_some_and(|body| parse_stats_says_invidious(&body))
        {
            return Some(Flavor::Invidious);
        }
    }
    if let Ok(res) = get(format!("{base}/suggestions?query=adios")).await {
        if res.status().is_success() && res.text().await.ok().is_some_and(|b| parse_is_str_list(&b))
        {
            return Some(Flavor::Piped);
        }
    }
    None
}

/// URL-encoding without pulling in a form library (space + RFC3986 set).
pub fn encode(term: &str) -> String {
    let mut out = String::with_capacity(term.len() * 2);
    for b in term.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn parse_stats_says_invidious(body: &str) -> bool {
    #[derive(Deserialize)]
    struct Stats {
        software: Software,
    }
    #[derive(Deserialize)]
    struct Software {
        name: String,
    }
    serde_json::from_str::<Stats>(body)
        .map(|s| s.software.name == "invidious")
        .unwrap_or(false)
}

pub fn parse_is_str_list(body: &str) -> bool {
    serde_json::from_str::<Vec<String>>(body).is_ok()
}

fn duration_ms_from_secs(secs: Option<f64>) -> Option<i64> {
    secs.map(|s| (s * 1000.0).round() as i64)
}

pub fn parse_piped_search(body: &str) -> Vec<SearchHit> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Item {
        url: String,
        #[serde(rename = "type")]
        kind: Option<String>,
        title: Option<String>,
        uploader_name: Option<String>,
        duration: Option<f64>,
    }
    #[derive(Deserialize)]
    struct Page {
        items: Vec<Item>,
    }
    let Ok(page) = serde_json::from_str::<Page>(body) else {
        return Vec::new();
    };
    page.items
        .into_iter()
        .filter(|item| item.kind.as_deref().is_none_or(|t| t == "stream"))
        .filter_map(|item| {
            let id = item.url.split("v=").nth(1)?.split('&').next()?.to_owned();
            Some(SearchHit {
                id,
                title: item.title?,
                uploader: item.uploader_name.unwrap_or_default(),
                duration_ms: duration_ms_from_secs(item.duration),
            })
        })
        .collect()
}

pub fn parse_invidious_search(body: &str) -> Vec<SearchHit> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Item {
        #[serde(rename = "type")]
        kind: Option<String>,
        video_id: String,
        title: String,
        author: Option<String>,
        length_seconds: Option<i64>,
    }
    let Ok(items) = serde_json::from_str::<Vec<Item>>(body) else {
        return Vec::new();
    };
    items
        .into_iter()
        .filter(|item| item.kind.as_deref().is_none_or(|t| t == "video"))
        .map(|item| SearchHit {
            id: item.video_id,
            title: item.title,
            uploader: item.author.unwrap_or_default(),
            duration_ms: item.length_seconds.map(|s| s * 1000),
        })
        .collect()
}

