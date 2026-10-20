//! P22/D35 — yt-dlp binary slot precedence: explicit env override >
//! the user's "Update yt-dlp" copy (settings) > the installer-bundled
//! copy (`bundle.resources`, tauri.windows.conf.json) > PATH lookup.
//! Precedence is pure (unit-tested); file probing happens in
//! `startup_path`, so stale settings / missing bundles fall through
//! instead of bricking the resolver.

use std::path::PathBuf;

/// `env` is trusted (explicit dev/test override, never probed);
/// `setting`/`bundled` must be pre-filtered to files that exist.
/// Final fallback is the bare name `yt-dlp`, resolved via PATH at spawn.
pub fn resolve_ytdlp_path(
    env: Option<PathBuf>,
    setting: Option<PathBuf>,
    bundled: Option<PathBuf>,
) -> PathBuf {
    env.or(setting)
        .or(bundled)
        .unwrap_or_else(|| PathBuf::from("yt-dlp"))
}

fn existing(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|p| p.is_file())
}

/// Startup wiring (lib.rs): env value, raw `ytdlpPath` JSON exactly as
/// stored in `settings.value_json`, bundled candidate — parse + probe,
/// then order. Invalid JSON and vanished files fall to the next slot.
pub fn startup_path(
    env: Option<String>,
    setting_raw: Option<String>,
    bundled: Option<PathBuf>,
) -> PathBuf {
    let setting = setting_raw
        .and_then(|raw| serde_json::from_str::<String>(&raw).ok())
        .map(PathBuf::from);
    resolve_ytdlp_path(env.map(PathBuf::from), existing(setting), existing(bundled))
}
