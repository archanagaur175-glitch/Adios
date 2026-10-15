//! P5.2 CAA tests: index parsing + pick preference + URL choice + sniffing
//! + UUID guard + 512² JPEG shaping. Fixtures are copied from the live API
//! docs (musicbrainz.org/doc/Cover_Art_Archive/API, fetched 2026-10-15).

use image::{DynamicImage, ImageFormat, Rgb};

use crate::meta::cover_art::{
    best_image_url, encode_cover_jpeg, image_ext, looks_like_uuid, pick_image, CaaIndex,
    CoverArtClient,
};
use crate::meta::palette;

/// Doc §"/release/{mbid}/ Example" — approved front with all thumbnails.
const RELEASE_INDEX: &str = r#"{
  "images": [
    {
      "types": ["Front"], "front": true, "back": false, "edit": 17462565,
      "image": "http://coverartarchive.org/release/76df3287-6cda-33eb-8e9a-044b5e15ffdd/829521842.jpg",
      "comment": "", "approved": true, "id": "829521842",
      "thumbnails": {
        "250": "http://coverartarchive.org/release/76df3287-6cda-33eb-8e9a-044b5e15ffdd/829521842-250.jpg",
        "500": "http://coverartarchive.org/release/76df3287-6cda-33eb-8e9a-044b5e15ffdd/829521842-500.jpg",
        "1200": "http://coverartarchive.org/release/76df3287-6cda-33eb-8e9a-044b5e15ffdd/829521842-1200.jpg",
        "small": "http://coverartarchive.org/release/76df3287-6cda-33eb-8e9a-044b5e15ffdd/829521842-250.jpg",
        "large": "http://coverartarchive.org/release/76df3287-6cda-33eb-8e9a-044b5e15ffdd/829521842-500.jpg"
      }
    }
  ],
  "release": "http://musicbrainz.org/release/76df3287-6cda-33eb-8e9a-044b5e15ffdd"
}"#;

/// Doc §"/release-group/{mbid}/ Example" — its only image is a BACK cover.
const RG_BACK_ONLY: &str = r#"{
  "release": "https://musicbrainz.org/release/f268b8bc-2768-426b-901b-c7966e76de29",
  "images": [
    {
      "edit": 37284546, "id": "12750224075",
      "image": "http://coverartarchive.org/release/f268b8bc-2768-426b-901b-c7966e76de29/12750224075.png",
      "thumbnails": {
        "250": "http://coverartarchive.org/release/f268b8bc/12750224075-250.jpg",
        "500": "http://coverartarchive.org/release/f268b8bc/12750224075-500.jpg",
        "1200": "http://coverartarchive.org/release/f268b8bc/12750224075-1200.jpg"
      },
      "comment": "", "approved": true, "front": false, "types": ["Back"], "back": true
    }
  ]
}"#;

#[test]
fn parses_doc_release_index_and_prefers_500_thumb() {
    let index: CaaIndex = serde_json::from_str(RELEASE_INDEX).expect("doc fixture");
    let pick = pick_image(&index).expect("front image");
    assert!(pick.front && pick.approved);
    assert_eq!(
        best_image_url(pick),
        "http://coverartarchive.org/release/76df3287-6cda-33eb-8e9a-044b5e15ffdd/829521842-500.jpg"
    );
}

#[test]
fn back_only_index_falls_back_to_first_image() {
    let index: CaaIndex = serde_json::from_str(RG_BACK_ONLY).expect("doc fixture");
    let pick = pick_image(&index).expect("fallback image");
    assert!(pick.back && !pick.front);
    assert!(best_image_url(pick).contains("-500.jpg"), "{}", best_image_url(pick));
}

#[test]
fn empty_index_has_no_pick() {
    let index: CaaIndex = serde_json::from_str(r#"{"images": []}"#).expect("fixture");
    assert!(pick_image(&index).is_none());
}

#[test]
fn magic_sniff_matches_formats_and_rejects_garbage() {
    assert_eq!(image_ext(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]), Some("jpg"));
    assert_eq!(
        image_ext(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0]),
        Some("png")
    );
    assert_eq!(image_ext(b"GIF89a...."), Some("gif"));
    assert_eq!(image_ext(b"RIFF\x00\x00\x00\x00WEBPVP8 "), Some("webp"));
    assert_eq!(image_ext(b"<html>404</html>"), None);
    assert_eq!(image_ext(b""), None);
}

#[test]
fn uuid_guard_accepts_doc_mbid_only() {
    assert!(looks_like_uuid("76df3287-6cda-33eb-8e9a-044b5e15ffdd"));
    assert!(looks_like_uuid("76DF3287-6CDA-33EB-8E9A-044B5E15FFDD"));
    assert!(!looks_like_uuid("76df32876cda33eb8e9a044b5e15ffdd"));
    assert!(!looks_like_uuid("not-a-uuid"));
    assert!(!looks_like_uuid(""));
}

#[test]
fn encode_cover_downscales_to_512_box_and_palette_roundtrips() {
    let img = image::RgbImage::from_fn(640, 640, |x, y| {
        Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    let mut png = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img)
        .write_to(&mut png, ImageFormat::Png)
        .expect("encode fixture png");
    let jpeg = encode_cover_jpeg(&png.into_inner()).expect("shape cover");
    assert_eq!(image_ext(&jpeg), Some("jpg"), "stored bytes are JPEG");
    let decoded = image::load_from_memory(&jpeg).expect("decode shaped cover");
    assert_eq!(decoded.width(), 512, "640² box scales down to 512");
    assert_eq!(decoded.height(), 512);
    let pal = palette::extract(&jpeg).expect("palette from stored artifact");
    assert_eq!(pal.colors.len(), 3);
    assert_eq!(pal.accent, pal.colors[0]);
}

/// Live network check (ignored by default): index + image + shaping +
/// palette for the MBID hard-coded in the CAA API docs.
/// Run: `cargo test caa_live -- --ignored --nocapture`
#[tokio::test]
#[ignore]
async fn caa_live_fetch_known_mbid() {
    let mbid = "76df3287-6cda-33eb-8e9a-044b5e15ffdd";
    let client = CoverArtClient::default();
    let (raw, url) = client.fetch_front(mbid).await.expect("live CAA front");
    let shaped = encode_cover_jpeg(&raw).expect("shape live cover");
    let pal = palette::extract(&shaped).expect("palette from live cover");
    println!(
        "[caa] live url={url} raw={}B → {}B accent={} colors={:?}",
        raw.len(),
        shaped.len(),
        pal.accent,
        pal.colors
    );
    assert_eq!(image_ext(&shaped), Some("jpg"));
    assert!(shaped.len() > 10_000, "shaped cover too small: {}", shaped.len());
}
