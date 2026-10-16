//! Pure yt-dlp arg construction (P4.2 + P9.7) — split from ytdlp.rs for
//! the 200-line budget. `build_args` and `strip_parentheticals` are pure
//! and unit-tested; the resolver itself imports them.

use super::Query;

/// Prefer m4a (AAC); fall back to any bestaudio (webm/Opus plays natively).
pub const FORMAT_SELECTOR: &str = "bestaudio[ext=m4a]/bestaudio";
/// yt-dlp network bound per request — keeps a hung instance from stalling.
pub const SOCKET_TIMEOUT_SECS: &str = "15";
/// P9.7: search three videos so a dead/region-blocked #1 doesn't fail the
/// whole resolve — the first entry that passes score() wins.
pub const SEARCH_COUNT: &str = "3";

/// P9.7: drop parenthetical/bracket segments — "Song (Live) (Deluxe Ed.)"
/// searched literally on YouTube finds nothing; "Song" finds the track.
/// An unclosed opener swallows the tail (best-effort).
pub fn strip_parentheticals(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for ch in s.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Pure arg builder — the spike command and unit tests share this.
pub fn build_args(query: &Query) -> Vec<String> {
    let artist = strip_parentheticals(&query.artist);
    let title = strip_parentheticals(&query.title);
    let search = format!("ytsearch{SEARCH_COUNT}:{artist} - {title}");
    vec![
        "--no-playlist".to_owned(),
        "--no-warnings".to_owned(),
        "--skip-download".to_owned(),
        "--socket-timeout".to_owned(),
        SOCKET_TIMEOUT_SECS.to_owned(),
        "--dump-single-json".to_owned(),
        "-f".to_owned(),
        FORMAT_SELECTOR.to_owned(),
        search,
    ]
}
