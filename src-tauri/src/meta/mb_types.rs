//! MusicBrainz search result types (P5.1).
//!
//! Shapes verified against the live docs 2026-10-15:
//! <https://musicbrainz.org/doc/MusicBrainz_API/Search> — artist/recording/
//! release JSON examples (score is a STRING like "100", `length` is ms,
//! keys `sort-name` / `artist-credit` / `first-release-date` / `life-span`
//! are kebab-case). `alias` maps those onto camelCase app fields so the
//! webview sees camelCase without a second struct layer.

use serde::de;
use serde::{Deserialize, Serialize};

/// Which MB entity endpoint a search targets (also names its cache kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    Artist,
    Recording,
    Release,
}

impl SearchKind {
    pub fn endpoint(self) -> &'static str {
        match self {
            SearchKind::Artist => "artist",
            SearchKind::Recording => "recording",
            SearchKind::Release => "release",
        }
    }

    /// `meta_cache.kind` column value for this search family.
    pub fn cache_kind(self) -> &'static str {
        match self {
            SearchKind::Artist => "mb_artist",
            SearchKind::Recording => "mb_recording",
            SearchKind::Release => "mb_release",
        }
    }

    /// Top-level JSON array key in the response body.
    pub fn list_key(self) -> &'static str {
        match self {
            SearchKind::Artist => "artists",
            SearchKind::Recording => "recordings",
            SearchKind::Release => "releases",
        }
    }
}

/// One page of results (camelCase for the webview).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults<T> {
    pub count: i64,
    pub offset: i64,
    pub items: Vec<T>,
}

/// Lenient `score` reader: MB serializes it as `"100"` (string), and some
/// fixtures use numbers. Missing/invalid → None instead of a hard error.
fn de_score<'de, D>(d: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct ScoreVisitor;
    impl<'de> de::Visitor<'de> for ScoreVisitor {
        type Value = Option<i32>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("score as string or number")
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(v.parse().ok())
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v as i32))
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as i32))
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            Ok(Some(v as i32))
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
    }
    d.deserialize_any(ScoreVisitor)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MbLifeSpan {
    #[serde(default)]
    pub begin: Option<String>,
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub ended: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MbArtist {
    pub id: String,
    #[serde(default, deserialize_with = "de_score")]
    pub score: Option<i32>,
    pub name: String,
    #[serde(default, alias = "sort-name")]
    pub sort_name: Option<String>,
    #[serde(default)]
    pub disambiguation: Option<String>,
    #[serde(default, alias = "life-span")]
    pub life_span: Option<MbLifeSpan>,
    #[serde(default, alias = "type")]
    pub artist_type: Option<String>,
}

/// One entry of an `artist-credit` array (recording/release results).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MbCredit {
    #[serde(default)]
    pub artist: Option<MbArtistRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MbArtistRef {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default, alias = "sort-name")]
    pub sort_name: Option<String>,
}

/// Flat artist-name list from a credit array (UI + enrichment matching).
#[allow(dead_code)] // consumed by the search UI (5.3) + import enrichment (5.4)
pub fn credit_names(credit: &Option<Vec<MbCredit>>) -> Vec<String> {
    credit
        .iter()
        .flatten()
        .filter_map(|c| c.artist.as_ref().map(|a| a.name.clone()))
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MbRecording {
    pub id: String,
    #[serde(default, deserialize_with = "de_score")]
    pub score: Option<i32>,
    pub title: String,
    /// Milliseconds (MB `length`).
    #[serde(default)]
    pub length: Option<i64>,
    #[serde(default, alias = "artist-credit")]
    pub artist_credit: Option<Vec<MbCredit>>,
    #[serde(default, alias = "first-release-date")]
    pub first_release_date: Option<String>,
}

impl MbRecording {
    #[allow(dead_code)] // search UI (5.3) + enrichment matching (5.4)
    pub fn artists(&self) -> Vec<String> {
        credit_names(&self.artist_credit)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MbRelease {
    pub id: String,
    #[serde(default, deserialize_with = "de_score")]
    pub score: Option<i32>,
    pub title: String,
    #[serde(default, alias = "artist-credit")]
    pub artist_credit: Option<Vec<MbCredit>>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

impl MbRelease {
    #[allow(dead_code)] // search UI (5.3) + enrichment matching (5.4)
    pub fn artists(&self) -> Vec<String> {
        credit_names(&self.artist_credit)
    }
}

/// Decode a full MB search body: `{created, count, offset, "<list>": [...]}`.
/// A missing list key decodes as an empty page (zero-result responses).
pub fn parse_list<T: serde::de::DeserializeOwned>(
    list_key: &str,
    body: &str,
) -> Result<SearchResults<T>, String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("mb: invalid json: {e}"))?;
    let count = v.get("count").and_then(|c| c.as_i64()).unwrap_or(0);
    let offset = v.get("offset").and_then(|c| c.as_i64()).unwrap_or(0);
    let items = match v.get(list_key) {
        Some(arr) if arr.is_array() => serde_json::from_value(arr.clone())
            .map_err(|e| format!("mb: invalid {list_key} items: {e}"))?,
        _ => Vec::new(),
    };
    Ok(SearchResults {
        count,
        offset,
        items,
    })
}
