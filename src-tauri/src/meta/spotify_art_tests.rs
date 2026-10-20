//! P21 tests: Spotify cover extraction — `visualIdentity.image[]` largest
//! wins, oEmbed URL/thumbnail helpers, and the honest None degradations.
//! Canned JSON only (the live shapes were probed 2026-10-19, see plan P21).

use crate::meta::spotify_embed::{
    oembed_thumbnail, oembed_url, pick_artwork, track_from_html,
};
use crate::meta::spotify_embed_types::EmbedImage;

const TID: &str = "FIXTUREAAAAAAAAAAAAA1";

fn img(url: Option<&str>, h: i64, w: i64) -> EmbedImage {
    EmbedImage { url: url.map(str::to_owned), max_height: Some(h), max_width: Some(w) }
}

fn track_html(entity: serde_json::Value) -> String {
    let next = serde_json::json!({
        "props": { "pageProps": { "state": { "data": { "entity": entity } } } }
    });
    format!(
        "<html><body><script id=\"__NEXT_DATA__\" type=\"application/json\">{next}</script></body></html>"
    )
}

#[test]
fn pick_artwork_prefers_largest_and_skips_urlless_entries() {
    let images = vec![
        img(Some("https://img/64"), 64, 64),
        img(Some("https://img/640"), 640, 640),
        img(Some("https://img/300"), 300, 300),
    ];
    assert_eq!(pick_artwork(&images).as_deref(), Some("https://img/640"));

    // A url-less 640 must not shadow the real 300.
    let with_hole = vec![img(None, 640, 640), img(Some("https://img/300"), 300, 300)];
    assert_eq!(pick_artwork(&with_hole).as_deref(), Some("https://img/300"));

    // Heights tie → width breaks it.
    let tied = vec![
        img(Some("https://img/narrow"), 300, 100),
        img(Some("https://img/wide"), 300, 300),
    ];
    assert_eq!(pick_artwork(&tied).as_deref(), Some("https://img/wide"));

    assert_eq!(pick_artwork(&[]), None);
    assert_eq!(pick_artwork(&[img(None, 640, 640)]), None);
}

#[test]
fn track_page_visual_identity_maps_largest_art() {
    let t = track_from_html(&track_html(serde_json::json!({
        "uri": format!("spotify:track:{TID}"),
        "name": "Song",
        "duration": 100000,
        "artists": [{ "name": "Artist" }],
        "visualIdentity": {
            "image": [
                { "url": "https://img/300", "maxHeight": 300, "maxWidth": 300 },
                { "url": "https://img/640", "maxHeight": 640, "maxWidth": 640 },
                { "url": "https://img/64", "maxHeight": 64, "maxWidth": 64 }
            ]
        }
    })))
    .expect("parse");
    assert_eq!(t.artwork_url.as_deref(), Some("https://img/640"));

    let no_vis = track_from_html(&track_html(serde_json::json!({
        "uri": format!("spotify:track:{TID}"),
        "name": "Song",
        "artists": [{ "name": "Artist" }]
    })))
    .expect("parse");
    assert_eq!(no_vis.artwork_url, None, "absent visualIdentity degrades");

    let empty_images = track_from_html(&track_html(serde_json::json!({
        "uri": format!("spotify:track:{TID}"),
        "name": "Song",
        "visualIdentity": { "image": [] }
    })))
    .expect("parse");
    assert_eq!(empty_images.artwork_url, None, "empty image list degrades");
}

#[test]
fn oembed_url_and_thumbnail() {
    assert_eq!(
        oembed_url("https://open.spotify.com", TID),
        format!("https://open.spotify.com/oembed?url=https://open.spotify.com/track/{TID}")
    );
    let body = r#"{"html":"<iframe…</iframe>","width":456,"height":152,
        "thumbnail_url":"https://img/art","thumbnail_width":300}"#;
    assert_eq!(oembed_thumbnail(body).as_deref(), Some("https://img/art"));
    assert_eq!(oembed_thumbnail(r#"{"thumbnail_url":"  "}"#), None);
    assert_eq!(oembed_thumbnail(r#"{"other":1}"#), None);
    assert_eq!(oembed_thumbnail("not json at all"), None);
}
