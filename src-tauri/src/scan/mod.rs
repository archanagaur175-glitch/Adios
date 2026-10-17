//! Folder scan: collect audio files, read tags, write covers, insert tracks (P3.2).
mod tags;

#[cfg(test)]
mod tests;

use crate::db::queries::{self, NewTrack};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::Instant;

const AUDIO_EXTS: &[&str] = &["aac", "flac", "m4a", "mp3", "ogg", "opus", "wav"];
/// Cap stored error strings so a broken folder cannot balloon the summary.
const MAX_ERRORS: usize = 50;

/// Result of one scan, surfaced to the UI (camelCase JSON).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub folder: String,
    pub added: u32,
    pub skipped: u32,
    pub failed: u32,
    pub errors: Vec<String>,
    pub elapsed_ms: u64,
}

/// True for supported audio file extensions (case-insensitive).
pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Recursively collect audio files (sorted); skips hidden entries (leading `.`).
pub fn collect_audio_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if is_audio_file(&path) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Deterministic, filesystem-safe cover name derived from the source path
/// (sanitized stem + FNV-1a of the full path — same file → same cover file).
pub fn cover_name(source: &Path) -> String {
    let stem: String = source
        .file_stem()
        .map(|s| {
            s.to_string_lossy()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                .collect()
        })
        .unwrap_or_else(|| "track".to_owned());
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in source.to_string_lossy().bytes() {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{stem}-{hash:016x}")
}

/// Import an explicit file list (used by tests and by `import_folder`).
pub fn import_files(
    conn: &Connection,
    folder: &str,
    files: &[PathBuf],
    covers_dir: &Path,
) -> ScanSummary {
    let started = Instant::now();
    let mut summary = ScanSummary {
        folder: folder.to_owned(),
        added: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
        elapsed_ms: 0,
    };
    for path in files {
        let path_str = path.to_string_lossy().into_owned();
        let meta = match tags::read_meta(path) {
            Ok(meta) => meta,
            Err(err) => {
                summary.failed += 1;
                if summary.errors.len() < MAX_ERRORS {
                    summary.errors.push(format!("{path_str}: {err}"));
                }
                continue;
            }
        };
        let cover_path = meta.cover.and_then(|cover| {
            let file = covers_dir.join(format!("{}.{}", cover_name(path), cover.ext));
            std::fs::write(&file, &cover.data)
                .ok()
                .map(|_| file.to_string_lossy().into_owned())
        });
        let existed = queries::track_exists(conn, &path_str).unwrap_or(false);
        let track = NewTrack {
            source: "local".to_owned(),
            path: Some(path_str.clone()),
            url: None,
            artist: meta.artist,
            title: meta.title,
            album: meta.album,
            duration_ms: Some(meta.duration_ms),
            year: meta.year,
            track_no: meta.track_no,
            disc_no: meta.disc_no,
            file_size: Some(meta.file_size),
            mtime: meta.mtime,
        };
        match queries::track_insert(conn, &track) {
            Ok(_) if existed => summary.skipped += 1,
            Ok(id) => {
                summary.added += 1;
                if let Some(cover) = cover_path {
                    queries::track_set_cover(conn, id, &cover).ok();
                }
            }
            Err(err) => {
                summary.failed += 1;
                if summary.errors.len() < MAX_ERRORS {
                    summary.errors.push(format!("{path_str}: {err}"));
                }
            }
        }
        // P16: `track_insert` never touches existing rows (ON CONFLICT DO
        // NOTHING), so backfill genre separately — fills NULLs only, idempotent.
        if let Some(genre) = meta.genre.as_deref().map(str::trim).filter(|g| !g.is_empty()) {
            queries::track_set_genre(conn, &path_str, genre).ok();
        }
    }
    summary.elapsed_ms = started.elapsed().as_millis() as u64;
    summary
}

/// Walk `folder` and import every audio file found.
pub fn import_folder(conn: &Connection, folder: &Path, covers_dir: &Path) -> ScanSummary {
    let files = collect_audio_files(folder);
    import_files(conn, &folder.to_string_lossy(), &files, covers_dir)
}
