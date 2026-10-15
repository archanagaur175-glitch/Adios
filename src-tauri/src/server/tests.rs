use super::*;
use axum::http::header::{HeaderValue, ORIGIN};
use std::io::Write;

/// One shared temp file per test: 200 KiB of a repeating byte pattern.
fn temp_file(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("adios-server-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    let data: Vec<u8> = (0..200 * 1024).map(|i| (i % 251) as u8).collect();
    file.write_all(&data).unwrap();
    path
}

fn app_origin() -> HeaderValue {
    HeaderValue::from_static("http://localhost:1420")
}

fn origin_headers(value: HeaderValue) -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(ORIGIN, value);
    headers
}

fn acao(res: &reqwest::Response) -> Option<String> {
    res.headers()
        .get(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

#[tokio::test]
async fn valid_request_returns_200_full_bytes_with_cors() {
    let server = start().expect("start");
    let path = temp_file("full.mp3");
    let url = server.media_url(&path);

    let res = reqwest::Client::new()
        .get(&url)
        .headers(origin_headers(app_origin()))
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert_eq!(acao(&res).as_deref(), Some("http://localhost:1420"));
    assert_eq!(res.headers().get("accept-ranges").unwrap(), "bytes");
    let body = res.bytes().await.unwrap();
    assert_eq!(body.len(), 200 * 1024);
    assert_eq!(body[0], 0);
    assert_eq!(body[100 * 1024], (100 * 1024 % 251) as u8);
}

#[tokio::test]
async fn range_request_returns_206_with_correct_slice() {
    let server = start().expect("start");
    let path = temp_file("range.mp3");
    let url = server.media_url(&path);

    let res = reqwest::Client::new()
        .get(&url)
        .header("range", "bytes=1000-1999")
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 206);
    assert_eq!(
        res.headers().get("content-range").unwrap(),
        "bytes 1000-1999/204800"
    );
    let body = res.bytes().await.unwrap();
    assert_eq!(body.len(), 1000);
    assert_eq!(body[0], (1000 % 251) as u8);
    assert_eq!(body[999], (1999 % 251) as u8);
}

#[tokio::test]
async fn wrong_token_returns_403() {
    let server = start().expect("start");
    let path = temp_file("token.mp3");
    let url = server.media_url(&path).replace(&server.token, "wrong-token");

    let res = reqwest::Client::new().get(&url).send().await.expect("request");
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn wrong_host_returns_403() {
    let server = start().expect("start");
    let path = temp_file("host.mp3");
    let url = server.media_url(&path);

    let res = reqwest::Client::new()
        .get(&url)
        .header("host", "evil.example:80")
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn missing_file_returns_404() {
    let server = start().expect("start");
    let url = server.media_url(std::path::Path::new("C:\\does\\not\\exist\\nope.mp3"));

    let res = reqwest::Client::new().get(&url).send().await.expect("request");
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn disallowed_origin_gets_no_cors_header() {
    let server = start().expect("start");
    let path = temp_file("origin.mp3");
    let url = server.media_url(&path);

    let res = reqwest::Client::new()
        .get(&url)
        .headers(origin_headers(HeaderValue::from_static("https://evil.example")))
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 200);
    assert_eq!(acao(&res), None, "non-app origin must not receive ACAO");
}

#[tokio::test]
async fn unsatisfiable_range_returns_416() {
    let server = start().expect("start");
    let path = temp_file("bad-range.mp3");
    let url = server.media_url(&path);

    let res = reqwest::Client::new()
        .get(&url)
        .header("range", "bytes=999999999-")
        .send()
        .await
        .expect("request");
    assert_eq!(res.status(), 416);
}
