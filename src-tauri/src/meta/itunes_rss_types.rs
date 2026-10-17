//! P16.5: RSS top-songs feed types + parser (split from `itunes_rss.rs`
//! for the 200-line budget). Feed shape verified live 2026-10-17
//! (`…/topsongs/limit=25/json`, HTTP 200): `feed.entry[]` →
//! `im:name.label` (name), `title.label` ("name - artist", suffix-stripped
//! fallback), `im:artist.label`, `im:image[]` (55/60/170 px, tallest wins),
//! `id.attributes["im:id"]`, `category.attributes.term` (genre),
//! `im:releaseDate.label`; entries without a numeric `im:id` are skipped
//! and there is no duration (the resolver takes `null`).

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Label {
    label: String,
}

#[derive(Debug, Deserialize)]
struct Image {
    label: String,
    attributes: Option<ImageAttrs>,
}

#[derive(Debug, Deserialize)]
struct ImageAttrs {
    height: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IdObj {
    attributes: Option<IdAttrs>,
}

#[derive(Debug, Deserialize)]
struct IdAttrs {
    #[serde(rename = "im:id")]
    im_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Category {
    attributes: Option<CategoryAttrs>,
}

#[derive(Debug, Deserialize)]
struct CategoryAttrs {
    term: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    #[serde(rename = "im:name")]
    name: Option<Label>,
    title: Label,
    #[serde(rename = "im:artist")]
    artist: Label,
    #[serde(rename = "im:image")]
    images: Option<Vec<Image>>,
    id: IdObj,
    category: Option<Category>,
    #[serde(rename = "im:releaseDate")]
    release_date: Option<Label>,
}

#[derive(Debug, Deserialize)]
struct Feed {
    entry: Option<Vec<Entry>>,
}

#[derive(Debug, Deserialize)]
struct Root {
    feed: Feed,
}

/// One chart row for the webview (camelCase, feeds `RecCandidate` directly).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RssSongOut {
    pub itunes_id: i64,
    pub title: String,
    pub artist: String,
    pub artwork_url: Option<String>,
    pub genre: Option<String>,
    pub release_date: Option<String>,
}

/// Decode a raw RSS body (pure, unit-tested). Feed order is chart order
/// (first = #1); `im:name` wins over the `title` suffix-strip fallback.
pub fn parse_feed(body: &str) -> Result<Vec<RssSongOut>, String> {
    let root: Root =
        serde_json::from_str(body).map_err(|e| format!("itunes_rss: bad json: {e}"))?;
    let mut out = Vec::new();
    for e in root.feed.entry.into_iter().flatten() {
        let Some(im_id) = e.id.attributes.as_ref().and_then(|a| a.im_id.as_deref()) else {
            continue;
        };
        let Ok(itunes_id) = im_id.parse::<i64>() else {
            continue;
        };
        let artist = e.artist.label.trim().to_owned();
        let title = match e.name.map(|n| n.label) {
            Some(n) if !n.trim().is_empty() => n,
            _ => {
                let full = &e.title.label;
                let suffix = format!(" - {artist}");
                full.strip_suffix(&suffix).unwrap_or(full).to_owned()
            }
        };
        let artwork_url = e.images.and_then(|imgs| {
            imgs.into_iter()
                .filter_map(|img| {
                    let h = img
                        .attributes
                        .as_ref()
                        .and_then(|a| a.height.as_deref())
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(0);
                    Some((h, img.label))
                })
                .max_by_key(|(h, _)| *h)
                .map(|(_, url)| url)
        });
        let genre = e
            .category
            .and_then(|c| c.attributes.and_then(|a| a.term))
            .filter(|g| !g.trim().is_empty());
        let release_date = e.release_date.map(|r| r.label);
        out.push(RssSongOut {
            itunes_id,
            title,
            artist,
            artwork_url,
            genre,
            release_date,
        });
    }
    Ok(out)
}
