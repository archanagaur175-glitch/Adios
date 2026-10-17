//! P13 stream-art tests: iTunes artwork fetch → cover file + palette on
//! the row; 600 px URL preferred with fallback; garbage never corrupts.

use image::{DynamicImage, ImageFormat, Rgb, RgbImage};

use crate::db::{stream, DbState};
use crate::meta::cover_art::CoverArtClient;
use crate::meta::stream_art;

fn block_jpeg() -> Vec<u8> {
    let img = RgbImage::from_fn(80, 40, |x, _| {
        if x < 32 {
            Rgb([255, 0, 0])
        } else if x < 60 {
            Rgb([0, 255, 0])
        } else {
            Rgb([0, 0, 255])
        }
    });
    let mut buf = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, ImageFormat::Jpeg)
        .expect("encode fixture jpeg");
    buf.into_inner()
}

fn fresh_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("adios-stream-art-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed_track(db: &crate::resolver::SharedDb) -> i64 {
    let conn = db.lock().unwrap();
    stream::stream_track_upsert(
        &conn,
        "https://example.invalid/song.m4a",
        "Artist",
        "Title",
        Some(180_000),
        Some("itunes:42"),
        Some("Pop"),
    )
    .expect("upsert")
    .id
}

fn cover_row(db: &crate::resolver::SharedDb, id: i64) -> (Option<String>, Option<String>) {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT cover_path, accent FROM tracks WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .expect("row")
}

#[test]
fn upscaled_url_swaps_the_size_segment() {
    let url = "https://is1-ssl.mzstatic.com/image/thumb/Music/x/100x100bb.jpg";
    assert_eq!(
        stream_art::upscaled_url(url),
        "https://is1-ssl.mzstatic.com/image/thumb/Music/x/600x600bb.jpg"
    );
    assert_eq!(stream_art::upscaled_url("https://x.invalid/a.png"), "https://x.invalid/a.png");
}

#[tokio::test]
async fn fetch_writes_cover_file_and_palette_on_the_row() {
    let jpeg = block_jpeg();
    let app = axum::Router::new().route(
        "/600x600bb.jpg",
        axum::routing::get(move || {
            let jpeg = jpeg.clone();
            async move {
                (
                    [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
                    jpeg,
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });

    let db = DbState::new(crate::db::open_memory().expect("db")).shared();
    let id = seed_track(&db);
    let dir = fresh_dir();
    let art_url = format!("http://{addr}/100x100bb.jpg");

    let path = stream_art::fetch_and_cache(&CoverArtClient::default(), &db, &dir, id, &art_url)
        .await
        .expect("fetch");

    assert!(path.ends_with(&format!("stream-{id}.jpg")));
    assert!(std::path::Path::new(&path).is_file(), "cover file on disk");
    let (cover, accent) = cover_row(&db, id);
    assert_eq!(cover.as_deref(), Some(path.as_str()), "cover_path on row");
    let accent = accent.expect("accent set");
    assert!(accent.starts_with('#'), "accent hex: {accent}");
}

#[tokio::test]
async fn falls_back_to_original_when_600_variant_404s() {
    let jpeg = block_jpeg();
    let app = axum::Router::new().route(
        "/100x100bb.jpg",
        axum::routing::get(move || {
            let jpeg = jpeg.clone();
            async move {
                (
                    [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
                    jpeg,
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });

    let db = DbState::new(crate::db::open_memory().expect("db")).shared();
    let id = seed_track(&db);
    let dir = fresh_dir();
    let art_url = format!("http://{addr}/100x100bb.jpg");

    let path = stream_art::fetch_and_cache(&CoverArtClient::default(), &db, &dir, id, &art_url)
        .await
        .expect("fallback fetch");
    assert!(std::path::Path::new(&path).is_file());
}

#[tokio::test]
async fn garbage_body_errors_and_leaves_the_row_untouched() {
    let app = axum::Router::new().route(
        "/100x100bb.jpg",
        axum::routing::get(|| async { "definitely not an image" }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });

    let db = DbState::new(crate::db::open_memory().expect("db")).shared();
    let id = seed_track(&db);
    let dir = fresh_dir();
    let art_url = format!("http://{addr}/100x100bb.jpg");

    let err = stream_art::fetch_and_cache(&CoverArtClient::default(), &db, &dir, id, &art_url)
        .await
        .unwrap_err();
    assert!(err.contains("caa"), "magic-byte rejection surfaces: {err}");
    let (cover, accent) = cover_row(&db, id);
    assert!(cover.is_none(), "no cover_path written");
    assert!(accent.is_none(), "no accent written");
}
