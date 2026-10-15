use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

/// HTTP Range handling for the loopback server (Invariant 1).

/// Inclusive byte range [start, end] of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRange {
    pub start: u64,
    pub end: u64,
}

/// Parse a single-range `Range: bytes=…` header against a file length.
/// Returns `Ok(None)` for no/unsupported range (serve full body),
/// `Some(range)` for a satisfiable range, `Err(416)` for unsatisfiable.
pub fn parse_range(header: Option<&str>, len: u64) -> Result<Option<FileRange>, ()> {
    let Some(header) = header else { return Ok(None) };
    let Some(spec) = header.strip_prefix("bytes=") else {
        return Err(()); // other units: ignore per RFC but we reject to be safe
    };
    // Only single ranges are supported; multipart ranges → full body.
    if spec.contains(',') {
        return Ok(None);
    }
    let Some((start_s, end_s)) = spec.split_once('-') else {
        return Err(());
    };
    let (start_s, end_s) = (start_s.trim(), end_s.trim());
    if start_s.is_empty() {
        // suffix range: bytes=-N → last N bytes
        let suffix: u64 = end_s.parse().map_err(|_| ())?;
        if suffix == 0 || len == 0 {
            return Err(());
        }
        let start = len.saturating_sub(suffix);
        return Ok(Some(FileRange {
            start,
            end: len - 1,
        }));
    }
    let start: u64 = start_s.parse().map_err(|_| ())?;
    if start >= len {
        return Err(());
    }
    let end = if end_s.is_empty() {
        len - 1
    } else {
        end_s.parse::<u64>().map_err(|_| ())?.min(len - 1)
    };
    if end < start {
        return Err(());
    }
    Ok(Some(FileRange { start, end }))
}

/// Best-effort content type from a file extension.
pub fn mime_for_path(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
        .as_str()
    {
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "m4a" | "mp4" => "audio/mp4",
        "aac" => "audio/aac",
        "ogg" | "oga" => "audio/ogg",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        "weba" | "webm" => "audio/webm",
        "wma" => "audio/x-ms-wma",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// Percent-encode a path for use as the `path` query value.
/// Used by `LoopbackServer::media_url` (tests / Rust-side callers).
#[allow(dead_code)]
pub fn encode_path(path: &str) -> String {
    utf8_percent_encode(path, NON_ALPHANUMERIC).to_string()
}
