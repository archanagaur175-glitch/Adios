//! P17.1 tests: url/id parsing, NEXT_DATA + token extraction, entity
//! shapes, spclient pagination JSON — plus a local axum end-to-end of
//! `fetch_full_playlist` (first page + token + extra pages + track meta).

use std::collections::HashMap;

use crate::meta::spotify_embed::{self as sp, EmbedBases};

const ID: &str = "37i9dQZF1DXcBWIGoYBM5M";
const T1: &str = "FIXTUREAAAAAAAAAAAAA1";
const T2: &str = "FIXTUREAAAAAAAAAAAAA2";
const TOKEN: &str = "tok_fixture_aaaaaaaaaaaaaaaaaa";

fn wrapper(entity: serde_json::Value) -> String {
    let next = serde_json::json!({
        "props": { "pageProps": { "state": {
            "data": { "entity": entity },
            "settings": { "session": { "accessToken": TOKEN } }
        } } }
    });
    format!("<html><body><script id=\"__NEXT_DATA__\" type=\"application/json\">{next}</script></body></html>")
}

fn playlist_html() -> String {
    wrapper(serde_json::json!({
        "name": "Fixture List",
        "trackList": [{ "uri": "spotify:track:FIXTUREAAAAAAAAAAAAA0", "title": "Song Zero", "subtitle": "Artist Z", "duration": 100000 }]
    }))
}

fn sp_json(from: i64) -> String {
    let items: Vec<serde_json::Value> = if from == 0 {
        vec![]
    } else {
        [T1, T2]
            .iter()
            .map(|t| serde_json::json!({ "uri": format!("spotify:track:{t}") }))
            .collect()
    };
    serde_json::json!({ "length": 3, "contents": { "pos": from, "truncated": false, "items": items } }).to_string()
}

#[test]
fn parse_playlist_id_accepts_url_uri_relative_and_bare() {
    for input in [
        format!("https://open.spotify.com/playlist/{ID}?si=abc"),
        format!("http://open.spotify.com/playlist/{ID}#frag"),
        format!("spotify:playlist:{ID}"),
        format!("/playlist/{ID}"),
        ID.to_owned(),
    ] {
        assert_eq!(sp::parse_playlist_id(&input).expect(&input), ID, "{input}");
    }
}

#[test]
fn parse_playlist_id_rejects_garbage_and_wrong_link_types() {
    for input in ["", "not a link", "https://example.com/x", "https://open.spotify.com/track/short", "spotify:track:aaaaaaaaaaaaaaaaaaaaaa"] {
        assert!(sp::parse_playlist_id(input).is_err(), "should err: {input}");
    }
}

#[test]
fn extract_next_data_and_access_token() {
    let html = playlist_html();
    let data = sp::extract_next_data(&html).expect("next data");
    assert!(data.contains("Fixture List"));
    assert_eq!(sp::extract_access_token(&html).as_deref(), Some(TOKEN));
    assert_eq!(sp::extract_next_data("<html>no script</html>"), None);
    assert_eq!(sp::extract_access_token("<html>no token</html>"), None);
}

#[test]
fn playlist_from_html_maps_entity_and_skips_non_tracks() {
    let html = wrapper(serde_json::json!({
        "name": "Mixed",
        "trackList": [
            { "uri": "spotify:track:FIXTUREAAAAAAAAAAAAA1", "title": "A", "subtitle": "X, Y", "duration": 120000 },
            { "uri": "spotify:episode:000000000000000000000A", "title": "pod", "subtitle": "s", "duration": 1 }
        ]
    }));
    let (name, tracks) = sp::playlist_from_html(&html).expect("parse");
    assert_eq!(name, "Mixed");
    assert_eq!(tracks.len(), 1, "episode filtered out");
    assert_eq!(tracks[0].artist, "X, Y");
    assert_eq!(tracks[0].duration_ms, Some(120000));
    let err = sp::playlist_from_html(&wrapper(serde_json::Value::Null)).unwrap_err();
    assert!(err.contains("not found or private"), "{err}");
    let err2 = sp::playlist_from_html("<html><body>nope</body></html>").unwrap_err();
    assert!(err2.contains("no data"), "{err2}");
}

#[test]
fn track_from_html_and_sp_json_roundtrip() {
    let html = wrapper(serde_json::json!({
        "uri": format!("spotify:track:{T1}"), "name": "Song One", "duration": 200000,
        "artists": [{ "name": "Artist A" }, { "name": "B" }]
    }));
    let t = sp::track_from_html(&html).expect("parse");
    assert_eq!(t.title, "Song One");
    assert_eq!(t.artist, "Artist A");
    assert_eq!(t.duration_ms, Some(200000));
    let (total, uris) = sp::sp_uris_from_json(&sp_json(1)).expect("sp");
    assert_eq!(total, 3);
    assert_eq!(uris, vec![format!("spotify:track:{T1}"), format!("spotify:track:{T2}")]);
}

#[tokio::test]
async fn fetch_full_playlist_end_to_end_against_fixture_server() {
    let app = axum::Router::new()
        .route("/embed/playlist/{id}", axum::routing::get(|| async { playlist_html() }))
        .route(
            "/embed/track/{tid}",
            axum::routing::get(
                |axum::extract::Path(tid): axum::extract::Path<String>| async move {
                    let (name, artist) = if tid == T1 { ("Song One", "Artist A") } else { ("Song Two", "Artist B") };
                    wrapper(serde_json::json!({
                        "uri": format!("spotify:track:{tid}"), "name": name, "duration": 200000,
                        "artists": [{ "name": artist }]
                    }))
                },
            ),
        )
        .route(
            "/playlist/v2/playlist/{id}",
            axum::routing::get(
                |axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>| async move {
                    sp_json(q.get("from").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0))
                },
            ),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
    let bases = EmbedBases {
        open: format!("http://{addr}"),
        spclient: format!("http://{addr}"),
    };

    let out = sp::fetch_full_playlist(&reqwest::Client::new(), &bases, &format!("https://open.spotify.com/playlist/{ID}?si=x"))
        .await
        .expect("fetch");
    assert_eq!(out.name, "Fixture List");
    assert_eq!(out.total, 3);
    assert_eq!(out.tracks.len(), 3, "first page + 2 track-page metas");
    assert_eq!(out.tracks[0].title, "Song Zero");
    assert_eq!(out.tracks[1].title, "Song One");
    assert_eq!(out.tracks[2].title, "Song Two");
    assert_eq!(out.tracks[2].duration_ms, Some(200000));
}
