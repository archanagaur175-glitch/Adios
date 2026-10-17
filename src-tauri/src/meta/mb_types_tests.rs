//! P5.1 tests: MB search body decoding. Fixtures are trimmed copies of the
//! JSON examples on musicbrainz.org/doc/MusicBrainz_API/Search (artist,
//! recording, release — verified 2026-10-15).

use crate::meta::mb_parse::parse_list;
use crate::meta::mb_types::{MbArtist, MbRecording, MbRelease, SearchKind};

const ARTIST_JSON: &str = r#"{
  "created": "2017-03-12T16:54:57.165Z",
  "count": 3,
  "offset": 0,
  "artists": [
    {
      "id": "e56fd97e-c18f-4e5e-9b4d-f9fc21b4973f",
      "type": "Group",
      "score": "100",
      "name": "Fred",
      "sort-name": "Fred",
      "country": "US",
      "disambiguation": "US progressive rock band",
      "life-span": { "begin": "1969", "end": "1974", "ended": true }
    },
    {
      "id": "1065e6e0-b6b3-4224-9121-4502e52b9f9e",
      "type": "Group",
      "score": 61,
      "name": "A Halo Called Fred",
      "sort-name": "Fred, A Halo Called"
    }
  ]
}"#;

const RECORDING_JSON: &str = r#"{
  "created": "2017-02-23T14:31:36.889Z",
  "count": 1,
  "offset": 0,
  "recordings": [
    {
      "id": "026fa041-3917-4c73-9079-ed16e36f20f8",
      "score": "100",
      "title": "Blow Your Mind (Mwah)",
      "length": 178000,
      "video": null,
      "artist-credit": [
        { "artist": { "id": "6f1a58bf-9b1b-49cf-a44a-6cefad7ae04f", "name": "Dua Lipa", "sort-name": "Lipa, Dua" } }
      ],
      "artist-credit-id": "4af83e24-f886-35c8-831f-5dde0163a6a0",
      "first-release-date": "2016-08-26"
    }
  ]
}"#;

const RELEASE_JSON: &str = r#"{
  "created": "2017-03-12T17:20:00.235Z",
  "count": 1,
  "offset": 0,
  "releases": [
    {
      "id": "62d1c4ef-fc00-37af-8df7-485f6a31fcc4",
      "score": "100",
      "count": 1,
      "title": "Fred Schneider & The Shake Society",
      "status": "Official",
      "artist-credit": [
        { "artist": { "id": "43bcca8b-9edc-4997-8343-122350e790bf", "name": "Fred Schneider", "sort-name": "Schneider, Fred" } }
      ],
      "date": "1987"
    }
  ]
}"#;

#[test]
fn artist_list_parses_doc_shape() {
    let page = parse_list::<MbArtist>(SearchKind::Artist.list_key(), ARTIST_JSON).unwrap();
    assert_eq!(page.count, 3);
    assert_eq!(page.items.len(), 2);
    let first = &page.items[0];
    assert_eq!(first.score, Some(100), "string score");
    assert_eq!(first.name, "Fred");
    assert_eq!(first.sort_name.as_deref(), Some("Fred"));
    assert_eq!(
        first.disambiguation.as_deref(),
        Some("US progressive rock band")
    );
    let span = first.life_span.as_ref().expect("life-span");
    assert_eq!(span.begin.as_deref(), Some("1969"));
    assert_eq!(span.ended, Some(true));
    assert_eq!(first.artist_type.as_deref(), Some("Group"));
    assert_eq!(page.items[1].score, Some(61), "numeric score tolerated");
}

#[test]
fn recording_list_parses_doc_shape() {
    let page = parse_list::<MbRecording>(SearchKind::Recording.list_key(), RECORDING_JSON).unwrap();
    assert_eq!(page.count, 1);
    let rec = &page.items[0];
    assert_eq!(rec.id, "026fa041-3917-4c73-9079-ed16e36f20f8");
    assert_eq!(rec.title, "Blow Your Mind (Mwah)");
    assert_eq!(rec.length, Some(178_000), "MB length is milliseconds");
    let names: Vec<String> = rec
        .artist_credit
        .iter()
        .flatten()
        .filter_map(|c| c.artist.as_ref().map(|a| a.name.clone()))
        .collect();
    assert_eq!(names, vec!["Dua Lipa".to_owned()]);
    assert_eq!(rec.first_release_date.as_deref(), Some("2016-08-26"));
    assert_eq!(rec.score, Some(100));
}

#[test]
fn release_list_parses_doc_shape() {
    let page = parse_list::<MbRelease>(SearchKind::Release.list_key(), RELEASE_JSON).unwrap();
    let rel = &page.items[0];
    assert_eq!(rel.title, "Fred Schneider & The Shake Society");
    let names: Vec<String> = rel
        .artist_credit
        .iter()
        .flatten()
        .filter_map(|c| c.artist.as_ref().map(|a| a.name.clone()))
        .collect();
    assert_eq!(names, vec!["Fred Schneider".to_owned()]);
    assert_eq!(rel.date.as_deref(), Some("1987"));
    assert_eq!(rel.status.as_deref(), Some("Official"));
    assert_eq!(rel.score, Some(100));
}

#[test]
fn garbage_body_is_an_error() {
    let err = parse_list::<MbArtist>(SearchKind::Artist.list_key(), "not json at all")
        .expect_err("garbage must fail");
    assert!(err.contains("invalid json"), "got: {err}");
}

#[test]
fn missing_list_key_is_an_empty_page() {
    let page = parse_list::<MbArtist>(SearchKind::Artist.list_key(), r#"{"count":0,"offset":0}"#)
        .expect("lenient decode");
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.count, 0);
}
