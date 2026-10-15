//! Cover Art Archive client + cover file shaping (P5.2).
//! Docs (fetched live 2026-10-15):
//! <https://musicbrainz.org/doc/Cover_Art_Archive/API> —
//! `GET /release/{mbid}/` → 307 → index JSON `{images[], release}`;
//! `images[].{front, back, approved, image, thumbnails{250|500|1200}}`;
//! image endpoints → 307 → binary. The docs' "Rate limiting rules"
//! section states CAA has **no** rate limits, so the MB 1-req/s queue does
//! not apply here. Stored file: `covers/<mbid>.jpg` downscaled to ≤ 512²
//! per docs/ARCHITECTURE.md; palette extraction is `super::palette`.

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CaaIndex {
    #[serde(default)]
    pub images: Vec<CaaImage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CaaImage {
    #[serde(default)]
    pub front: bool,
    /// Part of the documented shape; asserted in `cover_art_tests`
    /// (back-only releases exercise the pick fallback).
    #[allow(dead_code)]
    #[serde(default)]
    pub back: bool,
    #[serde(default)]
    pub approved: bool,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub thumbnails: HashMap<String, String>,
}

/// Approved front → any front → approved → first (back-only releases).
pub fn pick_image(index: &CaaIndex) -> Option<&CaaImage> {
    index
        .images
        .iter()
        .find(|i| i.front && i.approved)
        .or_else(|| index.images.iter().find(|i| i.front))
        .or_else(|| index.images.iter().find(|i| i.approved))
        .or_else(|| index.images.first())
}

/// 500 px thumbnail first (covers render ≤ 480 px), then 250/1200, then the
/// original — thumbnails are always `.jpg` per the docs.
pub fn best_image_url(image: &CaaImage) -> &str {
    for size in ["500", "250", "1200"] {
        if let Some(url) = image.thumbnails.get(size) {
            return url;
        }
    }
    &image.image
}

/// CAA answers 400 for anything that is not a UUID — fail fast client-side.
pub fn looks_like_uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

/// Magic-byte sniff (the stored extension must match the real bytes).
pub fn image_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpg");
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("png");
    }
    if bytes.starts_with(b"GIF8") {
        return Some("gif");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("webp");
    }
    None
}

/// Decode any supported cover, downscale to fit 512², encode JPEG — the
/// uniform bytes written to `covers/<mbid>.jpg`.
pub fn encode_cover_jpeg(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let rgb = image::load_from_memory(bytes)
        .map_err(|e| format!("caa: cover decode failed: {e}"))?
        .thumbnail(512, 512)
        .to_rgb8();
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(rgb)
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .map_err(|e| format!("caa: cover encode failed: {e}"))?;
    Ok(out.into_inner())
}

pub struct CoverArtClient {
    http: reqwest::Client,
}

impl Default for CoverArtClient {
    fn default() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }
}

impl CoverArtClient {
    /// JSON index: `GET /release/{mbid}/` (307 → index.json, followed).
    pub async fn fetch_index(&self, mbid: &str) -> Result<CaaIndex, String> {
        if !looks_like_uuid(mbid) {
            return Err(format!("caa: not a valid mbid: {mbid}"));
        }
        let url = format!("https://coverartarchive.org/release/{mbid}/");
        let res = self
            .http
            .get(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| format!("caa: index request failed: {e}"))?;
        match res.status() {
            s if s.is_success() => {
                let body = res
                    .text()
                    .await
                    .map_err(|e| format!("caa: index read failed: {e}"))?;
                serde_json::from_str(&body).map_err(|e| format!("caa: index decode failed: {e}"))
            }
            reqwest::StatusCode::NOT_FOUND => {
                Err("caa: no cover art for this release".to_owned())
            }
            s => Err(format!("caa: index HTTP {s}")),
        }
    }

    /// Image bytes behind the 307 redirect; rejects non-image bodies.
    pub async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        let res = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| format!("caa: image request failed: {e}"))?;
        if !res.status().is_success() {
            return Err(format!("caa: image HTTP {}", res.status()));
        }
        let bytes = res
            .bytes()
            .await
            .map_err(|e| format!("caa: image read failed: {e}"))?;
        if image_ext(&bytes).is_none() {
            return Err("caa: response is not a known image format".to_owned());
        }
        Ok(bytes.to_vec())
    }

    /// Index → best front → raw image bytes + source URL (for the log).
    pub async fn fetch_front(&self, mbid: &str) -> Result<(Vec<u8>, String), String> {
        let index = self.fetch_index(mbid).await?;
        let image =
            pick_image(&index).ok_or_else(|| "caa: index lists no images".to_owned())?;
        let url = best_image_url(image).to_owned();
        let bytes = self.fetch_bytes(&url).await?;
        Ok((bytes, url))
    }
}
