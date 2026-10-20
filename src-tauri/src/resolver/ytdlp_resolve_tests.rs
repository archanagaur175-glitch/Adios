//! P22/D35 — slot precedence: pure ordering + startup parsing/probing.

use std::path::PathBuf;

use crate::resolver::ytdlp_resolve::{resolve_ytdlp_path, startup_path};

fn slot(name: &str) -> Option<PathBuf> {
    Some(PathBuf::from(name))
}

#[test]
fn env_wins_over_setting_and_bundled() {
    assert_eq!(
        resolve_ytdlp_path(slot("env"), slot("setting"), slot("bundled")),
        PathBuf::from("env")
    );
}

#[test]
fn env_wins_even_alone() {
    assert_eq!(
        resolve_ytdlp_path(slot("env"), None, None),
        PathBuf::from("env")
    );
}

#[test]
fn setting_beats_bundled() {
    assert_eq!(
        resolve_ytdlp_path(None, slot("setting"), slot("bundled")),
        PathBuf::from("setting")
    );
}

#[test]
fn bundled_used_when_setting_absent() {
    assert_eq!(
        resolve_ytdlp_path(None, None, slot("bundled")),
        PathBuf::from("bundled")
    );
}

#[test]
fn bare_path_name_when_no_slots() {
    assert_eq!(
        resolve_ytdlp_path(None, None, None),
        PathBuf::from("yt-dlp")
    );
}

/// Unique-ish temp stub so parallel tests never collide.
fn temp_file(tag: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "adios-ytdlp-{tag}-{}-{}.bin",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, b"stub").expect("temp stub");
    path
}

#[test]
fn startup_env_is_trusted_without_file_probe() {
    // env points at a path that does NOT exist — still wins (explicit override).
    let got = startup_path(
        Some("C:/definitely/missing/env-yt-dlp.exe".into()),
        None,
        None,
    );
    assert_eq!(got, PathBuf::from("C:/definitely/missing/env-yt-dlp.exe"));
}

#[test]
fn startup_existing_setting_file_wins() {
    let setting = temp_file("setting");
    let raw = serde_json::to_string(setting.to_str().unwrap()).unwrap();
    let got = startup_path(None, Some(raw), None);
    assert_eq!(got, setting);
    let _ = std::fs::remove_file(&setting);
}

#[test]
fn startup_stale_setting_falls_through_to_bundled() {
    let bundled = temp_file("bundled");
    let stale = serde_json::to_string("C:/definitely/missing/yt-dlp.exe").unwrap();
    let got = startup_path(None, Some(stale), Some(bundled.clone()));
    assert_eq!(got, bundled);
    let _ = std::fs::remove_file(&bundled);
}

#[test]
fn startup_invalid_json_setting_is_ignored() {
    let bundled = temp_file("bundled-invalid");
    let got = startup_path(None, Some("not-json".into()), Some(bundled.clone()));
    assert_eq!(got, bundled);
    let _ = std::fs::remove_file(&bundled);
}

#[test]
fn startup_env_beats_existing_setting_and_bundled() {
    let setting = temp_file("setting-env");
    let raw = serde_json::to_string(setting.to_str().unwrap()).unwrap();
    let bundled = temp_file("bundled-env");
    let got = startup_path(
        Some("C:/env/yt-dlp.exe".into()),
        Some(raw),
        Some(bundled.clone()),
    );
    assert_eq!(got, PathBuf::from("C:/env/yt-dlp.exe"));
    let _ = std::fs::remove_file(&setting);
    let _ = std::fs::remove_file(&bundled);
}

#[test]
fn startup_vanished_bundled_falls_back_to_path_name() {
    let got = startup_path(None, None, Some(PathBuf::from("C:/missing/nope.exe")));
    assert_eq!(got, PathBuf::from("yt-dlp"));
}
