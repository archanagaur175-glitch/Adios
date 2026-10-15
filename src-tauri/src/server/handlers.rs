use crate::resolver::{SharedChain, SharedDb, SharedTable};
use crate::server::proxy::remote_response;
use crate::server::range::{mime_for_path, parse_range, FileRange};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header::{
    ACCEPT_RANGES, ACCESS_CONTROL_ALLOW_ORIGIN, ALLOW, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE,
    ORIGIN, RANGE,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, options};
use axum::Router;
use serde::Deserialize;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;

pub const ALLOWED_ORIGINS: &[&str] = &[
    "http://localhost:1420",  // vite dev
    "http://tauri.localhost", // Tauri v2 prod webview (Windows)
    "tauri://localhost",      // Tauri prod webview (macOS/Linux)
];

#[derive(Clone)]
pub struct ServerState {
    pub token: String,
    pub port: u16,
    pub client: reqwest::Client,
    pub db: SharedDb,
    pub cache_dir: PathBuf,
    /// Chain + table power the single re-resolve on upstream 4xx (P4.4).
    pub chain: SharedChain,
    pub table: SharedTable,
}

pub fn router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/media/{token}", get(media).merge(options(preflight)))
        .with_state(state)
}

#[derive(Deserialize)]
pub struct MediaQuery {
    pub path: Option<String>,
    /// Remote source (proxy + disk cache, P4.3). Mutually exclusive with path.
    pub url: Option<String>,
}

/// GET /media/:token?path=<local file> | ?url=<remote URL>
pub async fn media(
    State(state): State<Arc<ServerState>>,
    Path(token): Path<String>,
    Query(query): Query<MediaQuery>,
    headers: HeaderMap,
) -> Response {
    let origin = cors_origin(&headers);
    if token != state.token {
        return with_cors(plain(StatusCode::FORBIDDEN, "invalid token"), origin);
    }
    if !host_allowed(&headers, state.port) {
        return with_cors(plain(StatusCode::FORBIDDEN, "invalid host"), origin);
    }
    let range_header = headers.get(RANGE).and_then(|v| v.to_str().ok());

    if let Some(url) = query.url.as_deref() {
        let response = remote_response(&state, url, range_header).await;
        return with_cors(response, origin);
    }
    let Some(path) = query.path.as_deref() else {
        return with_cors(plain(StatusCode::BAD_REQUEST, "path or url required"), origin);
    };
    with_cors(path_response(&PathBuf::from(path), range_header).await, origin)
}

/// Local-file response (shared by ?path= requests and re-resolved paths).
pub(super) async fn path_response(path: &PathBuf, range_header: Option<&str>) -> Response {
    let meta = match tokio::fs::metadata(path).await {
        Ok(meta) if meta.is_file() => meta,
        _ => return plain(StatusCode::NOT_FOUND, "not found"),
    };
    let len = meta.len();
    match parse_range(range_header, len) {
        Ok(Some(range)) => file_response(path, len, Some(range)).await,
        Ok(None) => file_response(path, len, None).await,
        Err(()) => {
            let mut res = plain(StatusCode::RANGE_NOT_SATISFIABLE, "bad range");
            res.headers_mut().insert(
                CONTENT_RANGE,
                HeaderValue::from_str(&format!("bytes */{len}")).unwrap(),
            );
            res
        }
    }
}

/// OPTIONS preflight (defensive; media elements normally send simple requests).
pub async fn preflight(headers: HeaderMap) -> Response {
    let origin = cors_origin(&headers);
    let mut res = StatusCode::NO_CONTENT.into_response();
    res.headers_mut().insert(ALLOW, HeaderValue::from_static("GET, OPTIONS"));
    with_cors(res, origin)
}

pub(super) async fn file_response(path: &PathBuf, len: u64, range: Option<FileRange>) -> Response {
    let mime = mime_for_path(StdPath::new(path));
    let (status, start, end) = match range {
        Some(r) => (StatusCode::PARTIAL_CONTENT, r.start, r.end),
        None if len == 0 => (StatusCode::OK, 0, 0),
        None => (StatusCode::OK, 0, len - 1),
    };
    let body = if len == 0 {
        Body::empty()
    } else {
        super::stream::file_body(path.clone(), start, end)
    };
    let mut res = Response::new(body);
    *res.status_mut() = status;
    let h = res.headers_mut();
    h.insert(CONTENT_TYPE, HeaderValue::from_static(mime));
    h.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    h.insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&(end - start + 1).to_string()).unwrap(),
    );
    if status == StatusCode::PARTIAL_CONTENT {
        h.insert(
            CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{len}")).unwrap(),
        );
    }
    res
}

/// Host-header validation against DNS rebinding (Invariant 1).
pub fn host_allowed(headers: &HeaderMap, port: u16) -> bool {
    let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let host = host.trim().to_ascii_lowercase();
    host == format!("127.0.0.1:{port}")
        || host == format!("localhost:{port}")
        || host == "127.0.0.1"
        || host == "localhost"
}

/// Return the request origin only if it is on the app-origin allowlist.
pub fn cors_origin(headers: &HeaderMap) -> Option<HeaderValue> {
    let origin = headers.get(ORIGIN)?;
    if !origin
        .to_str()
        .map(|o| ALLOWED_ORIGINS.iter().any(|allowed| *allowed == o))
        .unwrap_or(false)
    {
        return None;
    }
    Some(origin.clone())
}

/// Attach CORS headers echoing the verified app origin (Invariant 1).
pub fn with_cors(mut res: Response, origin: Option<HeaderValue>) -> Response {
    if let Some(origin) = origin {
        res.headers_mut().insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    }
    res
}

fn plain(status: StatusCode, message: &'static str) -> Response {
    let mut res = Response::new(Body::from(message));
    *res.status_mut() = status;
    res
}
